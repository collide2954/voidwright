use std::fmt;
use std::io::{self, ErrorKind};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use voidwright_core::{Direction, ISLAND_RADIUS, Island, PlayerId, Tile, World};
use voidwright_protocol::{
    ClientMessage, Error, Inbox, MAX_SIGHTINGS, Name, Outbox, Reason, ServerMessage, Sighting,
    Update, Welcome,
};

const CAPACITY: usize = 2_000;
const GREETERS: usize = 64;
const GREETING: Duration = Duration::from_secs(5);
const VIEW: u16 = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub seed: u64,
    pub capacity: usize,
    pub greeting: Duration,
    pub view: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            seed: 1,
            capacity: CAPACITY,
            greeting: GREETING,
            view: VIEW,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub tick: u32,
    pub players: usize,
    pub messages: u64,
    pub refused: u64,
    pub dropped: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Close {
    Left,
    Refused(Reason),
    Malformed(Error),
    Fault(&'static str),
}

impl fmt::Display for Close {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => formatter.write_str("left"),
            Self::Refused(reason) => write!(formatter, "refused ({reason:?})"),
            Self::Malformed(error) => write!(formatter, "malformed data: {error}"),
            Self::Fault(fault) => formatter.write_str(fault),
        }
    }
}

enum Stage {
    Greeting {
        since: Instant,
    },
    Playing {
        id: PlayerId,
        name: Name,
        order: Option<Tile>,
    },
}

struct Host<'a> {
    world: &'a mut World,
    config: &'a Config,
    stats: &'a mut Stats,
    spawn: Tile,
}

struct Connection {
    stream: TcpStream,
    peer: SocketAddr,
    inbox: Inbox<ClientMessage>,
    outbox: Outbox,
    stage: Stage,
    close: Option<Close>,
}

impl Connection {
    fn new(stream: TcpStream, peer: SocketAddr) -> Self {
        Self {
            stream,
            peer,
            inbox: Inbox::default(),
            outbox: Outbox::default(),
            stage: Stage::Greeting {
                since: Instant::now(),
            },
            close: None,
        }
    }

    const fn greeting(&self) -> bool {
        matches!(self.stage, Stage::Greeting { .. })
    }

    fn receive(&mut self, host: &mut Host<'_>, now: Instant) {
        while self.close.is_none() {
            match self.inbox.pop() {
                Ok(Some(message)) => {
                    host.stats.messages += 1;
                    self.handle(message, host);
                    continue;
                }
                Ok(None) => {}
                Err(error) => {
                    self.reject(error);
                    break;
                }
            }
            match self.inbox.fill(&mut self.stream) {
                Ok(0) => self.close = Some(Close::Left),
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(_) => self.close = Some(Close::Left),
            }
        }
        if let Stage::Greeting { since } = self.stage
            && self.close.is_none()
            && now.saturating_duration_since(since) > host.config.greeting
        {
            self.close = Some(Close::Fault("no hello in time"));
        }
    }

    fn handle(&mut self, message: ClientMessage, host: &mut Host<'_>) {
        match message {
            ClientMessage::Hello(name) if self.greeting() => self.greet(name, host),
            ClientMessage::Hello(_) => self.close = Some(Close::Fault("a second hello")),
            ClientMessage::Walk(_) if self.greeting() => {
                self.close = Some(Close::Fault("a walk order before the hello"));
            }
            ClientMessage::Walk(target) => {
                if let Stage::Playing { order, .. } = &mut self.stage {
                    *order = Some(target);
                }
            }
        }
    }

    fn greet(&mut self, name: Name, host: &mut Host<'_>) {
        let room = host.world.population() < host.config.capacity;
        let Some(id) = room.then(|| host.world.join(host.spawn)).flatten() else {
            self.refuse(Reason::Full);
            return;
        };
        let welcome = Welcome {
            id,
            seed: host.config.seed,
            spawn: host.spawn,
        };
        if let Err(error) = self.outbox.push(&ServerMessage::Welcome(welcome)) {
            host.world.leave(id);
            self.close = Some(Close::Malformed(error));
            return;
        }
        println!("{} joins as {} {name}", self.peer, id.get());
        self.stage = Stage::Playing {
            id,
            name,
            order: None,
        };
    }

    fn refuse(&mut self, reason: Reason) {
        let refusal = self.outbox.push(&ServerMessage::Refuse(reason));
        self.close = Some(refusal.map_or_else(Close::Malformed, |()| Close::Refused(reason)));
    }

    fn reject(&mut self, error: Error) {
        let greeting = self.greeting();
        match error {
            Error::Version(_) if greeting => self.refuse(Reason::Version),
            Error::Name if greeting => self.refuse(Reason::Name),
            _ => self.close = Some(Close::Malformed(error)),
        }
    }

    fn take_order(&mut self) -> Option<(PlayerId, Tile)> {
        match &mut self.stage {
            Stage::Playing { id, order, .. } => order.take().map(|target| (*id, target)),
            Stage::Greeting { .. } => None,
        }
    }

    fn update(&mut self, world: &World, tick: u32, view: u16) {
        let Stage::Playing { id, .. } = &self.stage else {
            return;
        };
        let Some(me) = world.player(*id).filter(|_| self.close.is_none()) else {
            return;
        };
        let players = world
            .near(me.tile(), view)
            .into_iter()
            .take(MAX_SIGHTINGS)
            .map(|(id, walker)| Sighting {
                id,
                tile: walker.tile(),
                step: walker
                    .next_step()
                    .and_then(|next| Direction::between(walker.tile(), next)),
            })
            .collect();
        let update = ServerMessage::Update(Update { tick, players });
        if let Err(error) = self.outbox.push(&update) {
            self.close = Some(Close::Malformed(error));
        }
    }

    fn flush(&mut self) {
        if self.outbox.flush(&mut self.stream).is_err() && self.close.is_none() {
            self.close = Some(Close::Left);
        }
    }

    fn finish(self, stats: &mut Stats) {
        let close = self.close.unwrap_or(Close::Left);
        if matches!(close, Close::Malformed(_) | Close::Fault(_)) {
            stats.dropped += 1;
        }
        match self.stage {
            Stage::Playing { id, name, .. } => println!("{} {name} leaves: {close}", id.get()),
            Stage::Greeting { .. } => println!("{} closes: {close}", self.peer),
        }
    }
}

pub struct Server {
    listener: TcpListener,
    config: Config,
    world: World,
    spawn: Tile,
    connections: Vec<Connection>,
    stats: Stats,
}

impl Server {
    pub fn bind(address: impl ToSocketAddrs, config: Config) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        let island = Island::generate(config.seed, ISLAND_RADIUS);
        Ok(Self {
            listener,
            config,
            world: World::new(island.map),
            spawn: island.center,
            connections: Vec::new(),
            stats: Stats::default(),
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    #[must_use]
    pub const fn spawn(&self) -> Tile {
        self.spawn
    }

    #[must_use]
    pub fn stats(&self) -> Stats {
        Stats {
            players: self.world.population(),
            ..self.stats
        }
    }

    pub fn poll(&mut self) {
        self.accept();
        let now = Instant::now();
        let Self {
            connections,
            world,
            config,
            stats,
            spawn,
            ..
        } = self;
        let mut host = Host {
            world,
            config,
            stats,
            spawn: *spawn,
        };
        for connection in connections.iter_mut() {
            connection.receive(&mut host, now);
            connection.flush();
        }
        self.sweep();
    }

    pub fn tick(&mut self) {
        for connection in &mut self.connections {
            if let Some((id, target)) = connection.take_order()
                && self.world.try_walk_to(id, target).is_err()
            {
                self.stats.refused += 1;
            }
        }
        self.world.tick();
        self.stats.tick = self.stats.tick.wrapping_add(1);
        for connection in &mut self.connections {
            connection.update(&self.world, self.stats.tick, self.config.view);
            connection.flush();
        }
        self.sweep();
    }

    fn accept(&mut self) {
        let mut greeters = self
            .connections
            .iter()
            .filter(|connection| connection.greeting())
            .count();
        while greeters < GREETERS {
            match self.listener.accept() {
                Ok((stream, peer)) => {
                    if stream.set_nonblocking(true).is_ok() && stream.set_nodelay(true).is_ok() {
                        self.connections.push(Connection::new(stream, peer));
                        greeters += 1;
                    }
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error) => {
                    eprintln!("accept failed: {error}");
                    break;
                }
            }
        }
    }

    fn sweep(&mut self) {
        let Self {
            connections, stats, ..
        } = self;
        for connection in connections.extract_if(.., |connection| connection.close.is_some()) {
            connection.finish(stats);
        }
    }
}
