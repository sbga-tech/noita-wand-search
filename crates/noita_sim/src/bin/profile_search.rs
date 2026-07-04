#[cfg(feature = "profiling")]
fn main() {
    noita_sim::profiling::main();
}

#[cfg(not(feature = "profiling"))]
fn main() {
    eprintln!("profile_search requires: cargo run --release -p noita_sim --features profiling --bin profile_search -- [args]");
    std::process::exit(2);
}
