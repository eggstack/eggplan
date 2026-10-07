fn main() {
    // `std::env::args` panics on a non-UTF-8 argument, which would escape before
    // `run` could format anything; decode here and route the failure into the
    // normal diagnostic path instead.
    let mut args = Vec::new();
    for (index, argument) in std::env::args_os().skip(1).enumerate() {
        match argument.into_string() {
            Ok(argument) => args.push(argument),
            Err(_) => std::process::exit(eggplan_cli::report_invalid_argument(index)),
        }
    }
    std::process::exit(eggplan_cli::run(args));
}
