# Tug: Oxy's Package Manager

Open `crates/oxy-tug/src/`. The thing to notice first is the architecture: `tug` is its own crate,
separate from `oxy-core`, and it does *not* reimplement any of the language. It parses manifests,
resolves dependencies, and then delegates the actual running to the `oxy` binary through a flag.
That separation is deliberate and worth watching for as we read — it's why `oxy-core` knows nothing
about package registries and `tug` knows nothing about how the language executes. The crate is
small and split cleanly by job (manifest, lockfile, install, runner, scaffold), so this walkthrough
follows those files in turn, ending with the `--extern` mechanism that is the actual seam between
the two halves.

**Crate:** `crates/oxy-tug/`

Files:
- `src/manifest.rs` — `tug.toml` parsing
- `src/lockfile.rs` — `tug.lock` read/write
- `src/install.rs` — package download and installation
- `src/runner.rs` — `tug build`, `tug run`, `tug test`
- `src/scaffold.rs` — `tug new`, `tug init`
- `src/project.rs` — project root resolution
- `src/cli.rs` — command-line interface

---

## The manifest

```rust
// crates/oxy-tug/src/manifest.rs
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub package: PackageInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
    #[serde(default)]
    pub scripts: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    Simple(String),          // "1.0" — version constraint
    Detailed { version: String, path: Option<String>, git: Option<String> },
}
```

`toml::from_str` deserializes `tug.toml` directly into `Manifest`. The `#[serde(untagged)]`
on `DependencySpec` means TOML like `oxy-http = "1.0"` (simple string) and
`oxy-http = { version = "1.0", git = "..." }` (table) both deserialize correctly.

---

## The runner: how `tug run` works

```rust
// crates/oxy-tug/src/runner.rs
pub fn run(manifest: &Manifest, args: &[String]) -> Result<(), TugError> {
    let main_file = find_main_file(manifest)?;

    // Resolve all dependency paths
    let extern_paths = resolve_extern_paths(manifest)?;

    // Build the oxy-core run command
    let mut cmd = std::process::Command::new("oxy");
    cmd.arg("run");
    for (name, path) in &extern_paths {
        cmd.arg("--extern").arg(format!("{name}={path}"));
    }
    cmd.arg(&main_file);
    cmd.args(args);

    let status = cmd.status()?;
    if !status.success() {
        return Err(TugError::RunFailed);
    }
    Ok(())
}
```

`tug run` is not a reimplementation of the Oxy runtime. It resolves dependencies, builds
the `--extern` flags, and delegates to the `oxy` CLI binary. The `--extern name=path` flag
tells `oxy-core` to treat `name` as an external module located at `path`.

---

## The `--extern` mechanism

Oxy's core supports loading external modules via the `--extern` flag:

```bash
oxy run --extern http=/home/user/.tug/packages/oxy-http-1.0.3 main.ox
```

Inside `main.ox`:
```rust
use http::client;
```

The Oxy resolver sees `http` in the import path, checks the `--extern` table, finds
`/home/user/.tug/packages/oxy-http-1.0.3`, and loads `.ox` files from that directory.

This mechanism keeps `oxy-core` dependency-free — it does not know about package registries.
`tug` handles the registry; `oxy` handles the execution.

---

## `tug new`: scaffolding a project

```rust
// crates/oxy-tug/src/scaffold.rs
pub fn new_project(name: &str) -> Result<(), TugError> {
    // Create directory structure
    fs::create_dir_all(format!("{name}/src"))?;

    // Write tug.toml
    fs::write(format!("{name}/tug.toml"), formatdoc! {"
        [package]
        name = \"{name}\"
        version = \"0.1.0\"

        [dependencies]
    "})?;

    // Write src/main.ox
    fs::write(format!("{name}/src/main.ox"), "fn main() {\n    println(\"Hello from {name}!\");\n}\n")?;

    // Write .gitignore
    fs::write(format!("{name}/.gitignore"), ".tug/\n")?;

    println!("Created project '{name}'. Run: cd {name} && tug run");
    Ok(())
}
```

`tug new my-app` creates a ready-to-run project structure. The generated `main.ox` is
a hello-world that runs immediately with `tug run`.

---

## The CLI

```rust
// crates/oxy-tug/src/cli.rs
pub enum Command {
    New(String),      // tug new <name>
    Init,             // tug init (in current dir)
    Install,          // tug install
    Update,           // tug update
    Add(String),      // tug add <package>
    Run(Vec<String>), // tug run [args]
    Build,            // tug build
    Test(Vec<String>),// tug test [args]
    Publish,          // tug publish
}
```

Commands are parsed from `std::env::args()` and dispatched to the appropriate module.
The CLI is intentionally simple — no flags per command, no complex subcommand nesting.
`tug run` runs. `tug test` tests. `tug install` installs dependencies.
