mod common;

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
