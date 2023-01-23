fn main() {
    // if the `catr::get_args` function returns an `Ok(config)`
    // value, use `Result::and_then` to pass the `config`
    // to `catr::run`
    if let Err(e) = catr::get_args().and_then(catr::run) {
        // 👆 if either `get_args` or `run` returns an
        // `Err` print it to STDERR

        eprintln!("{}", e); // prints the error message to `STDERR`
        std::process::exit(1);
    }
}
