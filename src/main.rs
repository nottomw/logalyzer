// TODO: add some favicon

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    gui::run_gui(args);
}
