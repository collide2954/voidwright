mod common;

use common::{Client, server};
use voidwright_core::{ISLAND_RADIUS, Island};
use voidwright_protocol::{ClientMessage, Name, Reason, ServerMessage, VERSION};
use voidwright_server::Config;

#[test]
fn a_hello_gets_a_welcome_with_the_seed_and_the_spawn() {
    let mut server = server(Config {
        seed: 7,
        ..Config::default()
    });
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

fn hello(version: u16, name: &[u8]) -> Vec<u8> {
    let mut payload = vec![0x01];
    payload.extend_from_slice(&version.to_le_bytes());
    payload.push(u8::try_from(name.len()).unwrap());
    payload.extend_from_slice(name);
    let mut frame = u16::try_from(payload.len()).unwrap().to_le_bytes().to_vec();
    frame.extend(payload);
    frame
}

#[test]
fn a_hello_with_another_version_is_refused() {
    let mut server = server(Config::default());
    let mut client = Client::connect(&server);
    client.send_raw(&hello(VERSION + 1, b"Ash"));
    let refusal = client.receive(&mut server);
    assert_eq!(refusal, ServerMessage::Refuse(Reason::Version));
    assert!(client.is_closed(&mut server));
    assert_eq!(server.stats().players, 0);
}

#[test]
fn a_hello_with_a_bad_name_is_refused() {
    let mut server = server(Config::default());
    let mut client = Client::connect(&server);
    client.send_raw(&hello(VERSION, b"<script>"));
    let refusal = client.receive(&mut server);
    assert_eq!(refusal, ServerMessage::Refuse(Reason::Name));
    assert!(client.is_closed(&mut server));
}

#[test]
fn a_full_world_refuses_a_new_player() {
    let mut server = server(Config {
        capacity: 1,
        ..Config::default()
    });
    let (_ash, _) = Client::join(&mut server, "Ash");
    let mut bryn = Client::connect(&server);
    bryn.send(&ClientMessage::Hello(Name::new("Bryn").unwrap()));
    let refusal = bryn.receive(&mut server);
    assert_eq!(refusal, ServerMessage::Refuse(Reason::Full));
    assert!(bryn.is_closed(&mut server));
    assert_eq!(server.stats().players, 1);
}

#[test]
fn a_world_holds_2000_players() {
    assert_eq!(Config::default().capacity, 2_000);
}
