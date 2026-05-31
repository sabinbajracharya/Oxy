use std::process;

use colored::Colorize;

mod cli;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let exit = cli::dispatch(&args);
    process::exit(exit);
}

pub(crate) fn print_help() {
    println!("{} v{}", "tug".cyan().bold(), env!("CARGO_PKG_VERSION"));
    println!("{}\n", "The Oxy package manager.".italic());
    println!("{} tug [command] [options]\n", "Usage:".bold());
    println!("{}:", "Commands".bold());
    println!(
        "  {}        Create a new Oxy project in a new directory",
        "new <name>".cyan()
    );
    println!(
        "  {}                Initialize a project in the current directory",
        "init".cyan()
    );
    println!(
        "  {}             Add a dependency to tug.toml",
        "add <dep>".cyan()
    );
    println!(
        "  {}          Remove a dependency from tug.toml",
        "remove <dep>".cyan()
    );
    println!(
        "  {}      Update dependencies (optionally one by name)",
        "update [dep]".cyan()
    );
    println!(
        "  {}   Install a package from a path or URL into ~/.oxy/packages",
        "install <path|url>".cyan()
    );
    println!(
        "  {}     Remove an installed package",
        "uninstall <name>".cyan()
    );
    println!("  {}                List installed packages", "list".cyan());
    println!(
        "  {}               Build the project (compiles via oxy)",
        "build".cyan()
    );
    println!(
        "  {}        Run the project's main module",
        "run [-- args]".cyan()
    );
    println!(
        "  {}                Run all #[test] functions in the project\n",
        "test".cyan()
    );
    println!("{}:", "Options".bold());
    println!(
        "  {}           PrInt version information",
        "-V, --version".cyan()
    );
    println!(
        "  {}              PrInt this help message",
        "-h, --help".cyan()
    );
}
