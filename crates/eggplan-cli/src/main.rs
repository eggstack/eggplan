fn main() {
    std::process::exit(eggplan_cli::run(std::env::args().skip(1)));
}
