use voidwright_client::{App, Launch, Session};

fn main() {
    let launch = Launch::parse(std::env::args().skip(1));
    let session = launch.connect.as_deref().map(|address| {
        Session::connect(address, launch.player_name()).unwrap_or_else(|failure| {
            eprintln!("cannot join {address}: {failure}");
            std::process::exit(1);
        })
    });
    miniquad::start(App::conf(&launch), move || {
        Box::new(App::new(
            miniquad::window::new_rendering_backend(),
            launch,
            session,
        ))
    });
}
