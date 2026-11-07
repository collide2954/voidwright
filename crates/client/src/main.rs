use voidwright_client::App;

fn main() {
    miniquad::start(App::conf(), || {
        Box::new(App::new(miniquad::window::new_rendering_backend()))
    });
}
