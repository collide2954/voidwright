#![allow(dead_code)]

use std::io::{ErrorKind, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use voidwright_core::PlayerId;
use voidwright_protocol::{
    ClientMessage, Inbox, Name, Outbox, ServerMessage, Sighting, Update, Welcome,
};
use voidwright_server::{Config, Server};

pub const PATIENCE: Duration = Duration::from_secs(5);
const PAUSE: Duration = Duration::from_millis(1);

pub trait Pump {
    fn address(&self) -> SocketAddr;

    fn pump(&mut self);
}

impl Pump for Server {
    fn address(&self) -> SocketAddr {
        self.local_addr().unwrap()
    }

    fn pump(&mut self) {
        self.poll();
    }
}

pub fn server(config: Config) -> Server {
    Server::bind("127.0.0.1:0", config).unwrap()
}

pub fn settle(server: &mut Server, done: impl Fn(&Server) -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !done(server) {
        assert!(Instant::now() < deadline, "the server does not settle");
        server.poll();
        thread::sleep(PAUSE);
    }
}

pub struct Running {
    address: SocketAddr,
    flag: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Running {
    pub fn start(config: Config) -> Self {
        let mut server = server(config);
        let address = server.address();
        let flag = Arc::new(AtomicBool::new(true));
        let running = Arc::clone(&flag);
        let thread = thread::spawn(move || server.run(&running));
        Self {
            address,
            flag,
            thread: Some(thread),
        }
    }
}

impl Pump for Running {
    fn address(&self) -> SocketAddr {
        self.address
    }

    fn pump(&mut self) {}
}

impl Drop for Running {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

pub fn sighting(update: &Update, id: PlayerId) -> Sighting {
    update
        .players
        .iter()
        .find(|sighting| sighting.id == id)
        .copied()
        .expect("the update shows the player")
}

pub struct Client {
    stream: TcpStream,
    inbox: Inbox<ServerMessage>,
}

impl Client {
    pub fn connect(server: &impl Pump) -> Self {
        let stream = TcpStream::connect(server.address()).unwrap();
        stream.set_nonblocking(true).unwrap();
        stream.set_nodelay(true).unwrap();
        Self {
            stream,
            inbox: Inbox::default(),
        }
    }

    pub fn join(server: &mut impl Pump, name: &str) -> (Self, Welcome) {
        let mut client = Self::connect(server);
        client.send(&ClientMessage::Hello(Name::new(name).unwrap()));
        match client.receive(server) {
            ServerMessage::Welcome(welcome) => (client, welcome),
            other => panic!("no welcome: {other:?}"),
        }
    }

    pub fn send(&mut self, message: &ClientMessage) {
        let mut outbox = Outbox::default();
        outbox.push(message).unwrap();
        let deadline = Instant::now() + PATIENCE;
        while !outbox.is_empty() {
            outbox.flush(&mut self.stream).unwrap();
            assert!(Instant::now() < deadline, "the server does not read");
            thread::sleep(PAUSE);
        }
    }

    pub fn send_raw(&mut self, bytes: &[u8]) {
        self.stream.set_nonblocking(false).unwrap();
        self.stream.write_all(bytes).unwrap();
        self.stream.set_nonblocking(true).unwrap();
    }

    pub fn receive(&mut self, server: &mut impl Pump) -> ServerMessage {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if let Some(message) = self.inbox.pop().unwrap() {
                return message;
            }
            match self.inbox.fill(&mut self.stream) {
                Ok(0) => panic!("the server closed the connection"),
                Ok(_) => continue,
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) => panic!("the connection failed: {error}"),
            }
            assert!(Instant::now() < deadline, "no message from the server");
            server.pump();
            thread::sleep(PAUSE);
        }
    }

    pub fn update(&mut self, server: &mut impl Pump) -> Update {
        match self.receive(server) {
            ServerMessage::Update(update) => update,
            other => panic!("no update: {other:?}"),
        }
    }

    pub fn is_closed(&mut self, server: &mut impl Pump) -> bool {
        let deadline = Instant::now() + PATIENCE;
        while Instant::now() < deadline {
            while let Ok(Some(_)) = self.inbox.pop() {}
            match self.inbox.fill(&mut self.stream) {
                Ok(0) => return true,
                Ok(_) => continue,
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(_) => return true,
            }
            server.pump();
            thread::sleep(PAUSE);
        }
        false
    }
}
