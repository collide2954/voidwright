use std::thread;
use std::time::{Duration, Instant};

use voidwright_client::{Game, Session};
use voidwright_core::Tile;
use voidwright_server::{Config, Server};

const PATIENCE: Duration = Duration::from_secs(5);
const PAUSE: Duration = Duration::from_millis(1);

struct Player {
    session: Session,
    game: Game,
}

impl Player {
    fn join(server: &mut Server, name: &'static str) -> Self {
        let address = server.local_addr().unwrap().to_string();
        let connecting = thread::spawn(move || Session::connect(&address, name));
        let deadline = Instant::now() + PATIENCE;
        while !connecting.is_finished() {
            assert!(Instant::now() < deadline, "no welcome");
            server.poll();
            thread::sleep(PAUSE);
        }
        let session = connecting.join().unwrap().unwrap();
        let game = Game::online(session.welcome());
        Self { session, game }
    }

    fn follow(&mut self, server: &mut Server) {
        let deadline = Instant::now() + PATIENCE;
        while self.session.play(&mut self.game).unwrap() == 0 {
            assert!(Instant::now() < deadline, "no update");
            server.poll();
            thread::sleep(PAUSE);
        }
        self.game.update(Duration::from_millis(16));
    }
}

#[test]
fn two_games_walk_and_see_each_other_on_a_real_server() {
    let mut server = Server::bind("127.0.0.1:0", Config::default()).unwrap();
    let mut ash = Player::join(&mut server, "Ash");
    let mut bryn = Player::join(&mut server, "Bryn");
    let spawn = ash.game.world().player().tile();
    let target = Tile::new(spawn.x + 2, spawn.y);
    ash.game.walk_to(target);
    assert_eq!(ash.session.play(&mut ash.game).unwrap(), 0);
    let deadline = Instant::now() + PATIENCE;
    while server.stats().messages < 3 {
        assert!(Instant::now() < deadline, "the order does not arrive");
        server.poll();
        thread::sleep(PAUSE);
    }
    for _ in 0..3 {
        server.tick();
        ash.follow(&mut server);
        bryn.follow(&mut server);
    }
    assert_eq!(ash.game.world().player().tile(), target);
    assert_eq!(ash.game.marker(), None);
    assert_eq!(ash.game.others().len(), 1);
    assert_eq!(bryn.game.others().len(), 1);
}
