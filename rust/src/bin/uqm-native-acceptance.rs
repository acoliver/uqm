fn main() {
    if let Err(error) = uqm_rust::automation::native_runner::entry_with_arguments(
        &std::env::args().collect::<Vec<_>>(),
    ) {
        eprintln!("uqm-native-acceptance: {error}");
        std::process::exit(1);
    }
}
