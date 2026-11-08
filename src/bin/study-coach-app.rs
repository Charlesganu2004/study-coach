fn main() {
    if let Err(error) = study_coach::app::run(std::env::args().skip(1)) {
        eprintln!("Study Coach: {error}");
        std::process::exit(1);
    }
}
