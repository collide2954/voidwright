mod common;

use std::time::{Duration, Instant};

use common::{Client, server, settle};
use voidwright_server::Config;

#[test]
fn a_player_that_disconnects_leaves_the_world() {
    let mut server = server(Config::default());
    let (mut ash, _) = Client::join(&mut server, "Ash");
    let (bryn, gone) = Client::join(&mut server, "Bryn");
    drop(bryn);
    settle(&mut server, |server| server.stats().players == 1);
    server.tick();
    let update = ash.update(&mut server);
    assert!(update.players.iter().all(|sighting| sighting.id != gone.id));
}

#[test]
fn a_player_that_breaks_the_rules_leaves_the_world() {
    let mut server = server(Config::default());
    let (mut ash, _) = Client::join(&mut server, "Ash");
    ash.send_raw(&[0, 0]);
    assert!(ash.is_closed(&mut server));
    assert_eq!(server.stats().players, 0);
    assert_eq!(server.stats().dropped, 1);
}

#[test]
fn a_client_that_stops_reading_does_not_hold_up_the_tick() {
    let mut server = server(Config {
        backlog: 4096,
        ..Config::default()
    });
    let (mut ash, _) = Client::join(&mut server, "Ash");
    let (_bryn, _) = Client::join(&mut server, "Bryn");
    let mut slowest = Duration::ZERO;
    for tick in 1..=5_000 {
        let start = Instant::now();
        server.tick();
        slowest = slowest.max(start.elapsed());
        assert_eq!(ash.update(&mut server).tick, tick);
    }
    assert!(slowest < Duration::from_millis(100), "{slowest:?}");
}

#[test]
fn the_backlog_of_a_world_holds_64_kib() {
    assert_eq!(Config::default().backlog, 64 * 1024);
}
