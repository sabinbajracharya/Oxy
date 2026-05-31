//! Parsing and writing of `tug.toml` — the per-project package manifest.
//!
//! Schema (all keys quoted with `=` for TOML):
//!
//! ```toml
//! [package]
//! name = "myproj"
//! version = "0.1.0"
//!
//! [dependencies]
//! # short form: a version string against the (future) registry
//! some-pkg = "1.2.3"
//! # git dependency, with optional tag/rev/branch (mutually exclusive)
//! json = { git = "https://github.com/x/oxy-json", tag = "v1.0.0" }
//! # local path dependency
//! local = { path = "../local-pkg" }
//! ```

use toml::Value;

use crate::tug_err;
use crate::TugResult;

/// A parsed `tug.toml`.
#[derive(Debug, Clone)]
pub struct TugManifest {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<Dependency>,
}

/// A single dependency entry under `[dependencies]`.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub source: Source,
}

/// Where a dependency comes from.
#[derive(Debug, Clone)]
pub enum Source {
    /// `name = "1.2.3"` — resolved against the (future) Oxy registry.
    Version(String),
    /// `name = { git = "...", tag/rev/branch = "..." }`
    Git { url: String, reference: GitRef },
    /// `name = { path = "../local" }`
    Path(String),
}

/// Which git ref a Git dependency pins to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitRef {
    Tag(String),
    Rev(String),
    Branch(String),
    /// No ref specified — resolves to the default branch HEAD at install time.
    Default,
}

impl TugManifest {
    /// Create a minimal manifest with the given package metadata.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            dependencies: Vec::new(),
        }
    }

    /// Parse a `tug.toml` source string. Returns a human-readable error on
    /// malformed input or missing required fields.
    pub fn parse(source: &str) -> TugResult<Self> {
        let value: Value = source
            .parse::<Value>()
            .map_err(|e| format!("invalid TOML: {e}"))?;

        let pkg = value
            .get("package")
            .ok_or_else(|| "missing [package] table".to_string())?
            .as_table()
            .ok_or_else(|| "[package] must be a table".to_string())?;

        let name = pkg
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "[package] is missing required field `name`".to_string())?
            .to_string();

        let version = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "[package] is missing required field `version`".to_string())?
            .to_string();

        let mut dependencies = Vec::new();
        if let Some(deps) = value.get("dependencies") {
            let table = deps
                .as_table()
                .ok_or_else(|| "[dependencies] must be a table".to_string())?;
            for (dep_name, dep_value) in table {
                validate_dep_name(dep_name)?;
                dependencies.push(parse_dependency(dep_name, dep_value)?);
            }
        }

        Ok(Self {
            name,
            version,
            dependencies,
        })
    }

    /// Insert a dependency, replacing any existing entry with the same name.
    pub fn add_dependency(&mut self, dep: Dependency) {
        if let Some(slot) = self.dependencies.iter_mut().find(|d| d.name == dep.name) {
            *slot = dep;
        } else {
            self.dependencies.push(dep);
        }
    }

    /// Remove a dependency by name. Returns `true` if something was removed.
    pub fn remove_dependency(&mut self, name: &str) -> bool {
        let before = self.dependencies.len();
        self.dependencies.retain(|d| d.name != name);
        self.dependencies.len() != before
    }
}

impl std::fmt::Display for TugManifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[package]")?;
        writeln!(f, "name = {}", toml_string(&self.name))?;
        writeln!(f, "version = {}", toml_string(&self.version))?;
        if !self.dependencies.is_empty() {
            writeln!(f)?;
            writeln!(f, "[dependencies]")?;
            let mut sorted = self.dependencies.clone();
            sorted.sort_by(|a, b| a.name.cmp(&b.name));
            for dep in &sorted {
                writeln!(f, "{}", dep.serialize_line())?;
            }
        }
        Ok(())
    }
}

impl Dependency {
    pub fn version(name: impl Into<String>, ver: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: Source::Version(ver.into()),
        }
    }

    pub fn path(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: Source::Path(path.into()),
        }
    }

    pub fn git_tag(
        name: impl Into<String>,
        url: impl Into<String>,
        tag: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source: Source::Git {
                url: url.into(),
                reference: GitRef::Tag(tag.into()),
            },
        }
    }

    pub fn git_rev(
        name: impl Into<String>,
        url: impl Into<String>,
        rev: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source: Source::Git {
                url: url.into(),
                reference: GitRef::Rev(rev.into()),
            },
        }
    }

    /// Format this dependency as a single TOML line for `[dependencies]`.
    fn serialize_line(&self) -> String {
        let value = match &self.source {
            Source::Version(v) => toml_string(v),
            Source::Git { url, reference } => match reference {
                GitRef::Tag(t) => {
                    format!("{{ git = {}, tag = {} }}", toml_string(url), toml_string(t))
                }
                GitRef::Rev(r) => {
                    format!("{{ git = {}, rev = {} }}", toml_string(url), toml_string(r))
                }
                GitRef::Branch(b) => format!(
                    "{{ git = {}, branch = {} }}",
                    toml_string(url),
                    toml_string(b)
                ),
                GitRef::Default => format!("{{ git = {} }}", toml_string(url)),
            },
            Source::Path(p) => format!("{{ path = {} }}", toml_string(p)),
        };
        format!("{} = {value}", self.name)
    }
}

fn parse_dependency(name: &str, value: &Value) -> TugResult<Dependency> {
    match value {
        Value::String(s) => Ok(Dependency {
            name: name.to_string(),
            source: Source::Version(s.clone()),
        }),
        // The `toml` 0.8 crate parses inline tables (`{ k = "v" }`) as
        // `Value::Table` too, so a single arm covers both forms.
        Value::Table(t) => parse_table_dep(name, t),
        other => Err(tug_err!(
            "dependency `{name}` must be a version string or inline table, got {}",
            other.type_str()
        )),
    }
}

fn parse_table_dep(name: &str, t: &toml::map::Map<String, Value>) -> TugResult<Dependency> {
    let has_git = t.contains_key("git");
    let has_path = t.contains_key("path");
    let has_version = t.contains_key("version");

    let source_count = [has_git, has_path, has_version]
        .iter()
        .filter(|b| **b)
        .count();
    if source_count == 0 {
        return Err(tug_err!(
            "dependency `{name}` needs a source: one of `git`, `path`, or `version`"
        ));
    }
    if source_count > 1 {
        return Err(tug_err!(
            "dependency `{name}` can only specify one source (git, path, or version)"
        ));
    }

    if has_git {
        let url = t
            .get("git")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tug_err!("dependency `{name}`: `git` must be a string"))?
            .to_string();

        let tag = t.get("tag").and_then(|v| v.as_str());
        let rev = t.get("rev").and_then(|v| v.as_str());
        let branch = t.get("branch").and_then(|v| v.as_str());

        let refs_count = [tag, rev, branch].iter().filter(|o| o.is_some()).count();
        if refs_count > 1 {
            return Err(tug_err!(
                "dependency `{name}`: only one of `tag`, `rev`, or `branch` allowed"
            ));
        }
        let reference = if let Some(t) = tag {
            GitRef::Tag(t.to_string())
        } else if let Some(r) = rev {
            GitRef::Rev(r.to_string())
        } else if let Some(b) = branch {
            GitRef::Branch(b.to_string())
        } else {
            GitRef::Default
        };
        return Ok(Dependency {
            name: name.to_string(),
            source: Source::Git { url, reference },
        });
    }

    if has_path {
        let p = t
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("dependency `{name}`: `path` must be a string"))?
            .to_string();
        return Ok(Dependency {
            name: name.to_string(),
            source: Source::Path(p),
        });
    }

    // has_version is the only remaining option.
    let v = t
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("dependency `{name}`: `version` must be a string"))?
        .to_string();
    Ok(Dependency {
        name: name.to_string(),
        source: Source::Version(v),
    })
}

/// A dependency name must be a non-empty identifier-like string:
/// letters, digits, `-`, `_`. (No spaces, no path separators.)
fn validate_dep_name(name: &str) -> TugResult<()> {
    if name.is_empty() {
        return Err(tug_err!("dependency name must not be empty"));
    }
    for c in name.chars() {
        if !(c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(tug_err!(
                "invalid dependency name `{name}`: must be an identifier (letters, digits, `-`, `_`)"
            ));
        }
    }
    Ok(())
}

/// Render a string as a TOML basic string with escapes.
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
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
