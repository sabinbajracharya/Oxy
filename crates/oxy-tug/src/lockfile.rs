//! Parsing and writing of `tug.lock` — the resolved-dependency lockfile.
//!
//! Format (TOML):
//!
//! ```toml
//! version = 1
//!
//! [[package]]
//! name = "json"
//! source = "git+https://github.com/x/oxy-json"
//! resolved = "abc123..."     # commit SHA or path digest
//! checksum = "sha256:..."    # optional, future use
//! ```

use toml::Value;

use crate::tug_err;
use crate::TugResult;

/// Supported lockfile schema version. Bumped when the format changes
/// incompatibly. Older lockfiles are rejected with a clear error.
pub const LOCKFILE_VERSION: u64 = 1;

#[derive(Debug, Clone)]
pub struct TugLock {
    pub version: u64,
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone)]
pub struct LockedPackage {
    pub name: String,
    /// Where this package came from. Typically `git+<url>` or `path+<path>`.
    pub source: String,
    /// Exact resolved revision (git SHA, content hash, etc.).
    pub resolved: String,
    /// Optional content checksum for verification.
    pub checksum: Option<String>,
}

impl Default for TugLock {
    fn default() -> Self {
        Self {
            version: LOCKFILE_VERSION,
            packages: Vec::new(),
        }
    }
}

impl TugLock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(source: &str) -> TugResult<Self> {
        let value: Value = source
            .parse::<Value>()
            .map_err(|e| format!("invalid TOML in tug.lock: {e}"))?;

        let version = value
            .get("version")
            .and_then(|v| v.as_integer())
            .ok_or_else(|| "tug.lock is missing required field `version`".to_string())?;
        if version < 0 {
            return Err(tug_err!("invalid lockfile version: {version}"));
        }
        let version = version as u64;
        if version != LOCKFILE_VERSION {
            return Err(tug_err!(
                "unsupported lockfile version {version} (this tug supports version {LOCKFILE_VERSION})"
            ));
        }

        let mut packages = Vec::new();
        if let Some(arr) = value.get("package") {
            let arr = arr
                .as_array()
                .ok_or_else(|| "[[package]] entries must form an array of tables".to_string())?;
            for (idx, entry) in arr.iter().enumerate() {
                let t = entry
                    .as_table()
                    .ok_or_else(|| format!("[[package]] entry {idx} is not a table"))?;
                let name = t
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("[[package]] entry {idx} is missing `name`"))?
                    .to_string();
                let source = t
                    .get("source")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        format!("[[package]] `{name}` is missing required field `source`")
                    })?
                    .to_string();
                let resolved = t
                    .get("resolved")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        format!("[[package]] `{name}` is missing required field `resolved`")
                    })?
                    .to_string();
                let checksum = t
                    .get("checksum")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                packages.push(LockedPackage {
                    name,
                    source,
                    resolved,
                    checksum,
                });
            }
        }

        Ok(Self { version, packages })
    }

    /// Find a locked package by name.
    pub fn find(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// Insert or replace a locked package.
    pub fn upsert(&mut self, pkg: LockedPackage) {
        if let Some(slot) = self.packages.iter_mut().find(|p| p.name == pkg.name) {
            *slot = pkg;
        } else {
            self.packages.push(pkg);
        }
    }

    /// Remove a locked package by name. Returns true if removed.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.packages.len();
        self.packages.retain(|p| p.name != name);
        self.packages.len() != before
    }
}

impl std::fmt::Display for TugLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "version = {}", self.version)?;
        let mut sorted = self.packages.clone();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        for pkg in &sorted {
            writeln!(f)?;
            writeln!(f, "[[package]]")?;
            writeln!(f, "name = {}", toml_string(&pkg.name))?;
            writeln!(f, "source = {}", toml_string(&pkg.source))?;
            writeln!(f, "resolved = {}", toml_string(&pkg.resolved))?;
            if let Some(c) = &pkg.checksum {
                writeln!(f, "checksum = {}", toml_string(c))?;
            }
        }
        Ok(())
    }
}

fn toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
