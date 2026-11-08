use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use voidwright_core::{Rng, Tile};
use voidwright_protocol::{ClientMessage, Inbox, Name, Outbox, ServerMessage};
use voidwright_server::{Config, Server};

const TICKS: u32 = 50;
const BATCH: usize = 32;

struct Bot {
    stream: TcpStream,
    inbox: Inbox<ServerMessage>,
    outbox: Outbox,
}

impl Bot {
    fn connect(address: SocketAddr, index: usize) -> Self {
        let stream = TcpStream::connect(address).expect("the bot connects");
        stream
            .set_nonblocking(true)
            .expect("the bot does not block");
        let name = Name::new(&format!("bot {index}")).expect("the name obeys the rules");
        let mut outbox = Outbox::default();
        outbox
            .push(&ClientMessage::Hello(name))
            .expect("the hello encodes");
        Self {
            stream,
            inbox: Inbox::default(),
            outbox,
        }
    }

    fn pump(&mut self) {
        if self.outbox.flush(&mut self.stream).is_err() {
            return;
        }
        while matches!(self.inbox.fill(&mut self.stream), Ok(count) if count > 0) {
            while let Ok(Some(_)) = self.inbox.pop() {}
        }
    }
}

fn offset(rng: &mut Rng) -> i32 {
    i32::try_from(rng.next_u64() % 25).unwrap_or(0) - 12
}

fn main() {
    let players: usize = std::env::args()
        .nth(1)
        .and_then(|count| count.parse().ok())
        .unwrap_or(100);
    let mut server = Server::bind("127.0.0.1:0", Config::default()).expect("the server binds");
    let address = server.local_addr().expect("the server has an address");
    let spawn = server.spawn();
    let mut bots: Vec<Bot> = Vec::with_capacity(players);
    while bots.len() < players {
        let first = bots.len();
        let last = (first + BATCH).min(players);
        bots.extend((first..last).map(|index| Bot::connect(address, index)));
        while server.stats().players < bots.len() {
            bots.iter_mut().for_each(Bot::pump);
            server.poll();
        }
    }
    let mut rng = Rng::new(1);
    let mut times = Vec::new();
    for _ in 0..TICKS {
        for bot in &mut bots {
            if rng.next_u64().is_multiple_of(3) {
                let target = Tile::new(spawn.x + offset(&mut rng), spawn.y + offset(&mut rng));
                bot.outbox
                    .push(&ClientMessage::Walk(target))
                    .expect("the walk order encodes");
            }
            bot.pump();
        }
        server.poll();
        let start = Instant::now();
        server.tick();
        times.push(start.elapsed());
        bots.iter_mut().for_each(Bot::pump);
    }
    times.sort_unstable();
    let total: Duration = times.iter().sum();
    println!(
        "{players} players: mean tick {:?}, median {:?}, slowest {:?}, {} refused orders",
        total / TICKS,
        times[times.len() / 2],
        times[times.len() - 1],
        server.stats().refused,
    );
}
