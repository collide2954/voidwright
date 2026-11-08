use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

use voidwright_server::{Config, Options, Server, USAGE};

fn main() -> ExitCode {
    let options = match Options::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let config = Config {
        seed: options.seed,
        ..Config::default()
    };
    let mut server = match Server::bind(&options.address, config) {
        Ok(server) => server,
        Err(error) => {
            eprintln!("cannot listen on {}: {error}", options.address);
            return ExitCode::FAILURE;
        }
    };
    match server.local_addr() {
        Ok(address) => println!("listening on {address} with seed {}", options.seed),
        Err(error) => eprintln!("the address is not known: {error}"),
    }
    server.run(&AtomicBool::new(true));
    ExitCode::SUCCESS
}
