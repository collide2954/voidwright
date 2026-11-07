use voidwright_client::{App, Launch};

fn main() {
    let launch = Launch::parse(std::env::args().skip(1));
    miniquad::start(App::conf(), move || {
        Box::new(App::new(miniquad::window::new_rendering_backend(), launch))
    });
}
