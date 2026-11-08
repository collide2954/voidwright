use voidwright_client::catalog::Catalog;

fn main() {
    let words: Vec<String> = std::env::args().skip(1).collect();
    miniquad::start(Catalog::conf(), move || {
        Box::new(Catalog::new(
            miniquad::window::new_rendering_backend(),
            &words,
        ))
    });
}
