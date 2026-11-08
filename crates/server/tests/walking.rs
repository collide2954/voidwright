mod common;

use common::{Client, server, settle, sighting};
use voidwright_core::{Direction, Tile};
use voidwright_protocol::ClientMessage;
use voidwright_server::Config;

#[test]
fn each_tick_sends_each_player_an_update() {
    let mut server = server(Config::default());
    let (mut ash, welcome) = Client::join(&mut server, "Ash");
    server.tick();
    let first = ash.update(&mut server);
    server.tick();
    let second = ash.update(&mut server);
    assert_eq!(first.tick, server.stats().tick - 1);
    assert_eq!(second.tick, first.tick + 1);
    let me = sighting(&first, welcome.id);
    assert_eq!((me.tile, me.step), (welcome.spawn, None));
}

#[test]
fn each_update_shows_the_players_near_the_player() {
    let mut server = server(Config::default());
    let (mut ash, a) = Client::join(&mut server, "Ash");
    let (mut bryn, b) = Client::join(&mut server, "Bryn");
    server.tick();
    for update in [ash.update(&mut server), bryn.update(&mut server)] {
        assert_eq!(sighting(&update, a.id).tile, a.spawn);
        assert_eq!(sighting(&update, b.id).tile, b.spawn);
        assert_eq!(update.players.len(), 2);
    }
}

fn east(spawn: Tile, dx: i32) -> Tile {
    Tile::new(spawn.x + dx, spawn.y)
}

#[test]
fn a_walk_order_moves_the_player_one_tile_each_tick() {
    let mut server = server(Config::default());
    let (mut ash, welcome) = Client::join(&mut server, "Ash");
    ash.send(&ClientMessage::Walk(east(welcome.spawn, 3)));
    settle(&mut server, |server| server.stats().messages == 2);
    let seen: Vec<_> = (0..4)
        .map(|_| {
            server.tick();
            sighting(&ash.update(&mut server), welcome.id)
        })
        .collect();
    let tiles: Vec<Tile> = seen.iter().map(|me| me.tile).collect();
    let path: Vec<Tile> = [1, 2, 3, 3].map(|dx| east(welcome.spawn, dx)).to_vec();
    assert_eq!(tiles, path);
    assert_eq!(seen[0].step, Some(Direction::East));
    assert_eq!(seen[2].step, None);
}

#[test]
fn two_players_see_each_others_moves() {
    let mut server = server(Config::default());
    let (mut ash, a) = Client::join(&mut server, "Ash");
    let (mut bryn, b) = Client::join(&mut server, "Bryn");
    ash.send(&ClientMessage::Walk(east(a.spawn, 2)));
    bryn.send(&ClientMessage::Walk(east(b.spawn, -2)));
    settle(&mut server, |server| server.stats().messages == 4);
    server.tick();
    let seen_by_ash = ash.update(&mut server);
    let seen_by_bryn = bryn.update(&mut server);
    assert_eq!(sighting(&seen_by_bryn, a.id).tile, east(a.spawn, 1));
    assert_eq!(sighting(&seen_by_ash, b.id).tile, east(b.spawn, -1));
    assert_eq!(sighting(&seen_by_ash, b.id).step, Some(Direction::West));
}

#[test]
fn players_beyond_the_view_are_not_in_the_update() {
    let mut server = server(Config {
        view: 2,
        ..Config::default()
    });
    let (mut ash, a) = Client::join(&mut server, "Ash");
    let (mut bryn, b) = Client::join(&mut server, "Bryn");
    ash.send(&ClientMessage::Walk(east(a.spawn, 3)));
    settle(&mut server, |server| server.stats().messages == 3);
    let updates: Vec<_> = (0..3)
        .map(|_| {
            server.tick();
            ash.update(&mut server);
            bryn.update(&mut server)
        })
        .collect();
    let counts: Vec<usize> = updates.iter().map(|update| update.players.len()).collect();
    assert_eq!(counts, [2, 2, 1]);
    assert_eq!(sighting(&updates[2], b.id).tile, b.spawn);
}

#[test]
fn invalid_walk_orders_are_refused_and_the_player_stays() {
    let mut server = server(Config::default());
    let (mut ash, welcome) = Client::join(&mut server, "Ash");
    let targets = [Tile::new(0, 0), east(welcome.spawn, 100)];
    for (count, target) in (2_u64..).zip(targets) {
        ash.send(&ClientMessage::Walk(target));
        settle(&mut server, |server| server.stats().messages == count);
        server.tick();
        let me = sighting(&ash.update(&mut server), welcome.id);
        assert_eq!((me.tile, me.step), (welcome.spawn, None));
    }
    assert_eq!(server.stats().refused, 2);
}
