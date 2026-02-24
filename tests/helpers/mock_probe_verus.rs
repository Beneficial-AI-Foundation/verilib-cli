use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    let fixtures = env::var("MOCK_FIXTURES_DIR").unwrap_or_else(|_| {
        eprintln!("MOCK_FIXTURES_DIR not set");
        process::exit(1);
    });

    let subcommand = args.get(1).map(|s| s.as_str()).unwrap_or("");

    let output_path = args
        .windows(2)
        .find(|w| w[0] == "-o" || w[0] == "--output")
        .map(|w| &w[1]);

    let fixture_file = match subcommand {
        "tracked-csv" => "tracked_functions.csv",
        "stubify" => "stubs.json",
        "atomize" => "atoms.json",
        "specify" => "specs.json",
        "verify" => "proofs.json",
        _ => {
            eprintln!("mock-probe-verus: unknown subcommand '{}'", subcommand);
            process::exit(1);
        }
    };

    let src = format!("{}/{}", fixtures, fixture_file);
    if let Some(dest) = output_path {
        if let Some(parent) = std::path::Path::new(dest.as_str()).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::copy(&src, dest).unwrap_or_else(|e| {
            eprintln!(
                "mock-probe-verus: failed to copy {} -> {}: {}",
                src, dest, e
            );
            process::exit(1);
        });
    }
}
