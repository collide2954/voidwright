mod common;

use common::{Client, server};
use voidwright_core::{ISLAND_RADIUS, Island};
use voidwright_server::Config;

#[test]
fn a_hello_gets_a_welcome_with_the_seed_and_the_spawn() {
    let mut server = server(Config { seed: 7 });
    let (_client, welcome) = Client::join(&mut server, "Ash");
    assert_eq!(welcome.seed, 7);
    assert_eq!(welcome.spawn, Island::generate(7, ISLAND_RADIUS).center);
    assert_eq!(welcome.spawn, server.spawn());
    assert_eq!(server.stats().players, 1);
}

#[test]
fn each_player_gets_a_different_id() {
    let mut server = server(Config::default());
    let (_ash, first) = Client::join(&mut server, "Ash");
    let (_bryn, second) = Client::join(&mut server, "Bryn");
    assert_ne!(first.id, second.id);
    assert_eq!(server.stats().players, 2);
}
