use voidwright_client::inspect::{Inspector, Options};

fn main() {
    let options = Options::parse(std::env::args().skip(1));
    miniquad::start(Inspector::conf(), move || {
        Box::new(Inspector::new(
            miniquad::window::new_rendering_backend(),
            options,
        ))
    });
}
