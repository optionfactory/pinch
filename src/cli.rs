use std::fs::File;
use std::io::Write;

pub const DEFAULT_CONFIG_FILE: &str = "pinch.yaml";

pub fn parse_args() -> String {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("Pinch Supervisor v{}", env!("CARGO_PKG_VERSION"));
        println!("\nUsage: pinch [CONFIG_FILE] [OPTIONS]");
        println!("\nOptions:");
        println!("  -h, --help        Print this help message");
        println!("  -V, --version     Print version information");
        println!("  --init            Generate a default pinch.yaml in the current directory");
        println!("\nExamples:");
        println!("  pinch             ");
        println!("  pinch custom.yaml ");
        std::process::exit(0);
    }

    if args.iter().any(|arg| arg == "-V" || arg == "--version") {
        println!("Pinch v{}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    if args.iter().any(|arg| arg == "--init") {
        let default_yaml = include_str!("default_config.yaml");
        let path = DEFAULT_CONFIG_FILE;

        if std::path::Path::new(path).exists() {
            eprintln!("Error: '{}' already exists. We don't want to overwrite it!", path);
            std::process::exit(1);
        }

        let mut file = File::create(path).expect("Failed to create default config file");
        file.write_all(default_yaml.as_bytes()).expect("Failed to write config");

        println!("Successfully generated '{}'. You're ready to go!", path);
        std::process::exit(0);
    }

    if args.len() > 1 {
        args[1].clone()
    } else {
        DEFAULT_CONFIG_FILE.to_string()
    }
}
