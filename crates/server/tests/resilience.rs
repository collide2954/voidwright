mod common;

use std::time::{Duration, Instant};

use common::{Client, Running, server, settle};
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

#[test]
fn the_server_loop_keeps_its_pace_while_one_client_stalls_and_one_drops() {
    const TICK: Duration = Duration::from_millis(20);
    const TICKS: u32 = 50;
    let mut running = Running::start(Config {
        tick: TICK,
        ..Config::default()
    });
    let (mut ash, _) = Client::join(&mut running, "Ash");
    let (_bryn, _) = Client::join(&mut running, "Bryn");
    let (cara, _) = Client::join(&mut running, "Cara");
    let mut cara = Some(cara);
    let start = Instant::now();
    let mut last = ash.update(&mut running);
    for count in 0..TICKS {
        if count == TICKS / 2 {
            cara = None;
        }
        let update = ash.update(&mut running);
        assert_eq!(update.tick, last.tick + 1);
        last = update;
    }
    assert!(cara.is_none());
    let elapsed = start.elapsed();
    assert!(elapsed < TICK * TICKS * 3, "{elapsed:?}");
    assert_eq!(last.players.len(), 2);
}

#[test]
fn a_world_ticks_every_600_milliseconds() {
    assert_eq!(Config::default().tick, voidwright_core::TICK);
}
