use std::fmt;
use std::io::{self, ErrorKind};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};

use voidwright_core::{ISLAND_RADIUS, Island, PlayerId, Tile, World};
use voidwright_protocol::{
    ClientMessage, Error, Inbox, Name, Outbox, Reason, ServerMessage, Welcome,
};

const CAPACITY: usize = 2_000;
const GREETERS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub seed: u64,
    pub capacity: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            seed: 1,
            capacity: CAPACITY,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub players: usize,
    pub messages: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Close {
    Left,
    Refused(Reason),
    Malformed(Error),
}

impl fmt::Display for Close {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => formatter.write_str("left"),
            Self::Refused(reason) => write!(formatter, "refused ({reason:?})"),
            Self::Malformed(error) => write!(formatter, "malformed data: {error}"),
        }
    }
}

enum Stage {
    Greeting,
    Playing { id: PlayerId, name: Name },
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
            stage: Stage::Greeting,
            close: None,
        }
    }

    fn receive(&mut self, host: &mut Host<'_>) {
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
    }

    fn handle(&mut self, message: ClientMessage, host: &mut Host<'_>) {
        if let ClientMessage::Hello(name) = message
            && matches!(self.stage, Stage::Greeting)
        {
            self.greet(name, host);
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
        self.stage = Stage::Playing { id, name };
    }

    fn refuse(&mut self, reason: Reason) {
        let refusal = self.outbox.push(&ServerMessage::Refuse(reason));
        self.close = Some(refusal.map_or_else(Close::Malformed, |()| Close::Refused(reason)));
    }

    fn reject(&mut self, error: Error) {
        let greeting = matches!(self.stage, Stage::Greeting);
        match error {
            Error::Version(_) if greeting => self.refuse(Reason::Version),
            Error::Name if greeting => self.refuse(Reason::Name),
            _ => self.close = Some(Close::Malformed(error)),
        }
    }

    fn flush(&mut self) {
        if self.outbox.flush(&mut self.stream).is_err() && self.close.is_none() {
            self.close = Some(Close::Left);
        }
    }

    fn finish(self) {
        let close = self.close.unwrap_or(Close::Left);
        match self.stage {
            Stage::Playing { id, name } => println!("{} {name} leaves: {close}", id.get()),
            Stage::Greeting => println!("{} closes: {close}", self.peer),
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
            connection.receive(&mut host);
            connection.flush();
        }
        self.sweep();
    }

    fn accept(&mut self) {
        let mut greeters = self
            .connections
            .iter()
            .filter(|connection| matches!(connection.stage, Stage::Greeting))
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
        for connection in self
            .connections
            .extract_if(.., |connection| connection.close.is_some())
        {
            connection.finish();
        }
    }
}
