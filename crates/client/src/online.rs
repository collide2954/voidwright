use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, ErrorKind};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use glam::Vec2;
use voidwright_core::{PlayerId, TICK, Tile, Walker, World};
use voidwright_protocol::{
    ClientMessage, Error, Inbox, Name, Outbox, Reason, ServerMessage, Update, Welcome,
};

use crate::motion::Course;
use crate::{Game, Stride};

const PATIENCE: Duration = Duration::from_secs(5);
const READS: usize = 64;

#[derive(Debug)]
pub enum Failure {
    Name,
    Io(io::Error),
    Protocol(Error),
    Refused(Reason),
    Closed,
    Unexpected,
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name => formatter
                .write_str("a name has 1 to 12 letters, digits, spaces, hyphens, or underscores"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Protocol(error) => write!(formatter, "the server sent bad data: {error}"),
            Self::Refused(Reason::Version) => {
                formatter.write_str("the server uses a different protocol version")
            }
            Self::Refused(Reason::Full) => formatter.write_str("the world is full"),
            Self::Refused(Reason::Name) => formatter.write_str("the server refused the name"),
            Self::Closed => formatter.write_str("the server closed the connection"),
            Self::Unexpected => formatter.write_str("the server sent an unexpected message"),
        }
    }
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Self::Protocol(error)
    }
}

#[derive(Debug)]
pub struct Session {
    stream: TcpStream,
    welcome: Welcome,
    inbox: Inbox<ServerMessage>,
    outbox: Outbox,
}

impl Session {
    pub fn connect(address: &str, name: &str) -> Result<Self, Failure> {
        let name = Name::new(name).map_err(|_| Failure::Name)?;
        let mut stream = open(address)?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(PATIENCE))?;
        stream.set_write_timeout(Some(PATIENCE))?;
        let mut outbox = Outbox::default();
        outbox.push(&ClientMessage::Hello(name))?;
        outbox.flush(&mut stream)?;
        if !outbox.is_empty() {
            return Err(Failure::Io(ErrorKind::TimedOut.into()));
        }
        let mut inbox = Inbox::default();
        let welcome = loop {
            match inbox.pop()? {
                Some(ServerMessage::Welcome(welcome)) => break welcome,
                Some(ServerMessage::Refuse(reason)) => return Err(Failure::Refused(reason)),
                Some(ServerMessage::Update(_)) => return Err(Failure::Unexpected),
                None => {}
            }
            if inbox.fill(&mut stream)? == 0 {
                return Err(Failure::Closed);
            }
        };
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            welcome,
            inbox,
            outbox,
        })
    }

    #[must_use]
    pub const fn welcome(&self) -> &Welcome {
        &self.welcome
    }

    pub fn walk(&mut self, tile: Tile) -> Result<(), Failure> {
        self.outbox.push(&ClientMessage::Walk(tile))?;
        Ok(())
    }

    pub fn play(&mut self, game: &mut Game) -> Result<usize, Failure> {
        for tile in game.take_orders() {
            self.walk(tile)?;
        }
        let updates = self.exchange()?;
        for update in &updates {
            game.apply(update);
        }
        Ok(updates.len())
    }

    pub fn exchange(&mut self) -> Result<Vec<Update>, Failure> {
        self.outbox.flush(&mut self.stream)?;
        let mut updates = Vec::new();
        for _ in 0..READS {
            while let Some(message) = self.inbox.pop()? {
                match message {
                    ServerMessage::Update(update) => updates.push(update),
                    ServerMessage::Refuse(reason) => return Err(Failure::Refused(reason)),
                    ServerMessage::Welcome(_) => return Err(Failure::Unexpected),
                }
            }
            match self.inbox.fill(&mut self.stream) {
                Ok(0) if updates.is_empty() => return Err(Failure::Closed),
                Ok(0) => break,
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(updates)
    }
}

fn open(address: &str) -> Result<TcpStream, Failure> {
    let mut failure = io::Error::new(ErrorKind::NotFound, "the address has no host");
    for target in address.to_socket_addrs()? {
        match TcpStream::connect_timeout(&target, PATIENCE) {
            Ok(stream) => return Ok(stream),
            Err(error) => failure = error,
        }
    }
    Err(Failure::Io(failure))
}

#[derive(Clone, Copy, Debug, Default)]
struct Figure {
    course: Course,
    stride: Stride,
    at: Option<Vec2>,
}

#[derive(Clone, Debug, Default)]
pub struct Online {
    phase: Duration,
    goal: Option<Tile>,
    orders: Vec<Tile>,
    figures: BTreeMap<PlayerId, Figure>,
}

impl Online {
    #[must_use]
    pub fn alpha(&self) -> f32 {
        self.phase.div_duration_f32(TICK).min(1.0)
    }

    pub fn wait(&mut self, delta: Duration) {
        self.phase = (self.phase + delta).min(TICK);
    }

    #[must_use]
    pub const fn goal(&self) -> Option<Tile> {
        self.goal
    }

    pub fn order(&mut self, tile: Tile) {
        self.orders.push(tile);
        self.goal = Some(tile);
    }

    pub fn take_orders(&mut self) -> Vec<Tile> {
        std::mem::take(&mut self.orders)
    }

    pub fn apply(&mut self, update: &Update, world: &mut World, me: PlayerId, mine: &mut Course) {
        let progress = self.alpha();
        for sighting in &update.players {
            let next = sighting.step.map(|step| sighting.tile.step(step));
            let course = if sighting.id == me {
                &mut *mine
            } else {
                &mut self.figures.entry(sighting.id).or_default().course
            };
            if let Some(walker) = world.player(sighting.id) {
                follow(course, walker, sighting.tile, progress);
            }
            world.sync(sighting.id, sighting.tile, next);
        }
        let seen: BTreeSet<PlayerId> = update.players.iter().map(|sighting| sighting.id).collect();
        self.figures.retain(|id, _| {
            let stays = seen.contains(id);
            if !stays {
                world.leave(*id);
            }
            stays
        });
        if world.player(me).map(Walker::tile) == self.goal {
            self.goal = None;
        }
        self.phase = Duration::ZERO;
    }

    pub fn animate(&mut self, world: &World, seconds: f32, ticks: f32) {
        let progress = self.alpha();
        for (id, figure) in &mut self.figures {
            let Some(walker) = world.player(*id) else {
                continue;
            };
            figure.course.update(ticks);
            let at = figure
                .course
                .position(walker.trail(), walker.next_step(), progress);
            if let Some(before) = figure.at {
                figure.stride.update(before, at, seconds);
            }
            figure.at = Some(at);
        }
    }

    pub fn figures(&self) -> impl Iterator<Item = (Vec2, &Stride)> {
        self.figures
            .values()
            .filter_map(|figure| figure.at.map(|at| (at, &figure.stride)))
    }
}

fn follow(course: &mut Course, walker: &Walker, tile: Tile, progress: f32) {
    let entered = (tile != walker.tile()).then_some(tile);
    course.reroute(walker.trail(), walker.next_step(), entered, progress);
    course.advance(walker.trail());
}

#[cfg(test)]
mod tests {
    use super::{Failure, Session};
    use std::net::{TcpListener, TcpStream};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};
    use voidwright_core::{PlayerId, Tile};
    use voidwright_protocol::{
        ClientMessage, Inbox, Name, Outbox, Reason, ServerMessage, Update, Welcome,
    };

    const WELCOME: Welcome = Welcome {
        id: PlayerId::new(3),
        seed: 9,
        spawn: Tile::new(28, 28),
    };

    struct Fake {
        stream: TcpStream,
        inbox: Inbox<ClientMessage>,
    }

    impl Fake {
        fn receive(&mut self) -> ClientMessage {
            loop {
                if let Some(message) = self.inbox.pop().unwrap() {
                    return message;
                }
                assert!(self.inbox.fill(&mut self.stream).unwrap() > 0);
            }
        }

        fn send(&mut self, message: &ServerMessage) {
            let mut outbox = Outbox::default();
            outbox.push(message).unwrap();
            outbox.flush(&mut self.stream).unwrap();
        }
    }

    fn serve(script: impl FnOnce(Fake) + Send + 'static) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            script(Fake {
                stream,
                inbox: Inbox::default(),
            });
        });
        (address, server)
    }

    fn welcomed(mut fake: Fake) -> Fake {
        fake.receive();
        fake.send(&ServerMessage::Welcome(WELCOME));
        fake
    }

    #[test]
    fn connect_says_hello_and_reads_the_welcome() {
        let (address, server) = serve(|mut fake| {
            let hello = ClientMessage::Hello(Name::new("Ash").unwrap());
            assert_eq!(fake.receive(), hello);
            fake.send(&ServerMessage::Welcome(WELCOME));
        });
        let session = Session::connect(&address, "Ash").unwrap();
        assert_eq!(session.welcome(), &WELCOME);
        server.join().unwrap();
    }

    #[test]
    fn connect_gives_the_reason_of_a_refusal() {
        let (address, server) = serve(|mut fake| {
            fake.receive();
            fake.send(&ServerMessage::Refuse(Reason::Full));
        });
        let failure = Session::connect(&address, "Ash").err();
        assert!(
            matches!(failure, Some(Failure::Refused(Reason::Full))),
            "{failure:?}"
        );
        server.join().unwrap();
    }

    #[test]
    fn connect_checks_the_name_before_it_connects() {
        let failure = Session::connect("127.0.0.1:9", "<b>").err();
        assert!(matches!(failure, Some(Failure::Name)), "{failure:?}");
    }

    fn exchange_until(session: &mut Session, count: usize) -> Vec<Update> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut updates = Vec::new();
        while updates.len() < count {
            updates.extend(session.exchange().unwrap());
            assert!(Instant::now() < deadline, "no update");
            thread::sleep(Duration::from_millis(1));
        }
        updates
    }

    #[test]
    fn a_session_sends_walk_orders_and_gives_the_updates() {
        let update = Update {
            tick: 4,
            players: Vec::new(),
        };
        let sent = update.clone();
        let (address, server) = serve(move |fake| {
            let mut fake = welcomed(fake);
            assert_eq!(fake.receive(), ClientMessage::Walk(Tile::new(30, 28)));
            fake.send(&ServerMessage::Update(sent.clone()));
            fake.send(&ServerMessage::Update(sent));
        });
        let mut session = Session::connect(&address, "Ash").unwrap();
        session.walk(Tile::new(30, 28)).unwrap();
        assert_eq!(exchange_until(&mut session, 2), [update.clone(), update]);
        server.join().unwrap();
    }

    #[test]
    fn a_session_reports_the_end_of_the_connection() {
        let (address, server) = serve(|fake| drop(welcomed(fake)));
        let mut session = Session::connect(&address, "Ash").unwrap();
        server.join().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match session.exchange() {
                Err(Failure::Closed | Failure::Io(_)) => break,
                Ok(_) => assert!(Instant::now() < deadline, "the session stays open"),
                Err(other) => panic!("{other}"),
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}
