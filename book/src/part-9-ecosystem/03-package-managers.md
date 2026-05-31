# Package Managers: The Boring Essential

<!-- OPUS_FILL
Write a 2-paragraph hook. Package managers are the unsexy part of a language ecosystem.
Nobody writes blog posts about them. But they are the thing that makes a language ecosystem
function — without a package manager, every Oxy program would be one file with no dependencies.

Reference Cargo (Rust's package manager) as the gold standard. Then frame tug as a simpler
version: a manifest file, a lock file, install packages, run programs. The 80% that matters.
-->

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
