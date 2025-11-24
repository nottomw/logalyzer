// TODO: lua support maybe
// TODO: stream support

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    gui::run_gui(args);
}
