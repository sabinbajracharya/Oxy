use std::path::Path;

use colored::Colorize;

use oxy_tug::{install, project, runner, scaffold};

/// Dispatch the CLI. Returns the process exit code.
pub fn dispatch(args: &[String]) -> i32 {
    match args.get(1).map(|s| s.as_str()) {
        Some("--version" | "-V") => {
            println!("tug v{}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some("--help" | "-h") | None => {
            crate::print_help();
            0
        }
        Some("new") => cmd_new(&args[2..]),
        Some("init") => cmd_init(&args[2..]),
        Some("install") => cmd_install(&args[2..]),
        Some("uninstall") => cmd_uninstall(&args[2..]),
        Some("list") => cmd_list(),
        Some("add") => cmd_add(&args[2..]),
        Some("remove" | "rm") => cmd_remove(&args[2..]),
        Some("run") => cmd_run(&args[2..]),
        Some("test") => cmd_test(),
        Some("build") => cmd_build(),
        Some(cmd) => {
            eprintln!("{} unknown command '{cmd}'", "error:".red().bold());
            eprintln!("Run 'tug --help' for usage information.");
            2
        }
    }
}

fn cmd_new(args: &[String]) -> i32 {
    let Some(name) = args.first() else {
        eprintln!("{} 'new' requires a project name", "error:".red().bold());
        return 2;
    };
    let target = Path::new(name);
    match scaffold::new_project(target, name) {
        Ok(()) => {
            println!(
                "{} created project {} in {}",
                "success:".green().bold(),
                name.cyan(),
                target.display()
            );
            0
        }
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            1
        }
    }
}

fn cmd_install(args: &[String]) -> i32 {
    let Some(target) = args.first() else {
        eprintln!("{} 'install' requires a path or URL", "error:".red().bold());
        return 2;
    };
    println!("{} installing from '{}'...", "info:".cyan(), target);
    let result = if target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("git@")
        || target.contains(':')
    {
        install::install_from_url(target)
    } else {
        let path = Path::new(target);
        if path.exists() && path.is_dir() {
            install::install_from_path(path)
        } else {
            Err(oxy_tug::tug_err!("package source not found: '{target}'"))
        }
    };
    match result {
        Ok(pkg) => {
            println!(
                "{} installed {} v{}",
                "success:".green().bold(),
                pkg.manifest.name.cyan().bold(),
                pkg.manifest.version
            );
            println!("  path: {}", pkg.path.display());
            0
        }
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            1
        }
    }
}

fn cmd_uninstall(args: &[String]) -> i32 {
    let Some(name) = args.first() else {
        eprintln!(
            "{} 'uninstall' requires a package name",
            "error:".red().bold()
        );
        return 2;
    };
    match install::uninstall(name) {
        Ok(path) => {
            println!("{} uninstalled {}", "success:".green().bold(), name.cyan());
            println!("  removed: {}", path.display());
            0
        }
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            1
        }
    }
}

fn cmd_list() -> i32 {
    match install::list_installed() {
        Ok(pkgs) => {
            let dir = install::packages_dir();
            if pkgs.is_empty() {
                println!(
                    "{} no packages installed in {}",
                    "info:".cyan(),
                    dir.display()
                );
                return 0;
            }
            println!(
                "{} {} installed in {}\n",
                "info:".cyan(),
                if pkgs.len() == 1 {
                    "1 package".to_string()
                } else {
                    format!("{} packages", pkgs.len())
                },
                dir.display(),
            );
            for pkg in &pkgs {
                println!(
                    "  {} {}",
                    pkg.manifest.name.cyan().bold(),
                    format!("v{}", pkg.manifest.version).dimmed()
                );
                println!("    {}", pkg.path.display().to_string().dimmed());
            }
            0
        }
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            1
        }
    }
}

fn cmd_add(args: &[String]) -> i32 {
    // tug add <spec> [--git <url>] [--tag <t>|--rev <r>] [--path <p>]
    let mut spec: Option<String> = None;
    let mut git: Option<String> = None;
    let mut tag: Option<String> = None;
    let mut rev: Option<String> = None;
    let mut path: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--git" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("{} --git requires a URL", "error:".red().bold());
                    return 2;
                };
                git = Some(v.clone());
                i += 2;
            }
            "--tag" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("{} --tag requires a value", "error:".red().bold());
                    return 2;
                };
                tag = Some(v.clone());
                i += 2;
            }
            "--rev" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("{} --rev requires a value", "error:".red().bold());
                    return 2;
                };
                rev = Some(v.clone());
                i += 2;
            }
            "--path" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("{} --path requires a value", "error:".red().bold());
                    return 2;
                };
                path = Some(v.clone());
                i += 2;
            }
            other if spec.is_none() => {
                spec = Some(other.to_string());
                i += 1;
            }
            other => {
                eprintln!("{} unexpected argument: '{other}'", "error:".red().bold());
                return 2;
            }
        }
    }
    let Some(spec) = spec else {
        eprintln!(
            "{} 'add' requires a dependency name (optionally `name@version`)",
            "error:".red().bold()
        );
        return 2;
    };
    let dep = match project::parse_dep_spec(&spec, git, tag, rev, path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    let mut proj = match project::Project::find(&cwd) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    let name = dep.name.clone();
    if let Err(e) = proj.add_dependency(dep) {
        eprintln!("{} {}", "error:".red().bold(), e);
        return 1;
    }
    println!(
        "{} added {} to {}/tug.toml",
        "success:".green().bold(),
        name.cyan().bold(),
        proj.root().display()
    );
    0
}

fn cmd_remove(args: &[String]) -> i32 {
    let Some(name) = args.first() else {
        eprintln!(
            "{} 'remove' requires a dependency name",
            "error:".red().bold()
        );
        return 2;
    };
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    let mut proj = match project::Project::find(&cwd) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    match proj.remove_dependency(name) {
        Ok(true) => {
            println!(
                "{} removed {} from tug.toml",
                "success:".green().bold(),
                name.cyan()
            );
            0
        }
        Ok(false) => {
            eprintln!(
                "{} no dependency named '{name}' in tug.toml",
                "error:".red().bold()
            );
            1
        }
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            1
        }
    }
}

fn cmd_run(args: &[String]) -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    let proj = match project::Project::find(&cwd) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    match runner::run_project(&proj, args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            1
        }
    }
}

fn cmd_test() -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    let proj = match project::Project::find(&cwd) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    match runner::test_project(&proj) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            1
        }
    }
}

fn cmd_build() -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    let proj = match project::Project::find(&cwd) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            return 1;
        }
    };
    match runner::build_project(&proj) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            1
        }
    }
}

fn cmd_init(args: &[String]) -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "{} could not read current directory: {e}",
                "error:".red().bold()
            );
            return 1;
        }
    };
    // Optional --name override; otherwise default to basename of cwd.
    let mut name = String::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--name" {
            let Some(n) = args.get(i + 1) else {
                eprintln!("{} '--name' requires a value", "error:".red().bold());
                return 2;
            };
            name = n.clone();
            i += 2;
            continue;
        }
        eprintln!(
            "{} unexpected argument to 'init': {}",
            "error:".red().bold(),
            args[i]
        );
        return 2;
    }
    match scaffold::init_project(&cwd, &name) {
        Ok(()) => {
            println!(
                "{} initialized project in {}",
                "success:".green().bold(),
                cwd.display()
            );
            0
        }
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            1
        }
    }
}
