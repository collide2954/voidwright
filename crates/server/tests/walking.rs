mod common;

use common::{Client, server, sighting};
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
