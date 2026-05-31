# Package Managers: The Boring Essential

Nobody writes a breathless blog post about a package manager. There are no conference talks titled
"The Beauty of Lockfiles." It is the unglamorous plumbing of a language ecosystem — and it is also,
quietly, the thing that decides whether an ecosystem exists at all. Without a package manager,
every Oxy program is an island: one file, no dependencies, no way to use anyone else's code or
share your own. The moment you want to build on top of someone else's HTTP library, you need a
manifest that declares it, a resolver that finds a compatible version, a place to download it from,
and a lockfile so your teammate gets the exact same thing you did. That machinery is boring, and it
is load-bearing.

Rust's Cargo is the gold standard here — version resolution, workspaces, features, the works — and
it's the obvious thing to point at. Oxy's package manager, `tug`, is deliberately a smaller animal.
No workspaces, no feature flags, no elaborate version solver. What it has is the 80% that actually
matters day to day: a `tug.toml` manifest to declare dependencies, a `tug.lock` to pin them, a way
to install them, and a way to run and test your project. This chapter is about that core loop —
what each piece is for and, especially, why the lockfile is the part you can't skip.

## What a package manager does

A package manager:
1. **Declares dependencies** — `tug.toml` says "this project needs `oxy-http` v1.2"
2. **Resolves dependencies** — finds compatible versions of everything
3. **Downloads and installs** — fetches packages from a registry
4. **Locks versions** — `tug.lock` pins exact versions so builds are reproducible
5. **Runs commands** — `tug build`, `tug run`, `tug test`

Oxy's package manager is `tug`. It is simpler than Cargo — no workspace support, no
feature flags, no complex version resolution — but it handles the core workflow.

## The `tug.toml` manifest

```toml
[package]
name = "my-app"
version = "0.1.0"

[dependencies]
oxy-http = "1.0"
oxy-json = "0.5"

[scripts]
start = "run src/main.ox"
test = "test src/"
```

`tug.toml` is the project description. The `[dependencies]` section declares what the
project needs; `[scripts]` are named shortcuts for common commands.

## Why lock files matter

```toml
# tug.lock (auto-generated, commit to VCS)
[[package]]
name = "oxy-http"
version = "1.0.3"   # ← exact version, not just "1.0"
checksum = "sha256:abc123..."

[[package]]
name = "oxy-json"
version = "0.5.1"
checksum = "sha256:def456..."
```

Without a lock file: two developers run `tug install` on different days and get different
versions. One gets `oxy-http 1.0.3` (today's latest), the other gets `1.0.4` (tomorrow's
release). The bug introduced in `1.0.4` affects one but not the other.

With a lock file: both get exactly `1.0.3`. Reproducible builds everywhere.

The lock file is committed to version control. `tug install` uses the lock file if it exists.
`tug update` refreshes the lock file to the latest compatible versions.
