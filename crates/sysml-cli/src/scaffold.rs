//! `rsysdsl new`: scaffold a compiling, audit-clean, lint-clean model crate with the
//! layered `Types -> Requirements -> Subsystems -> Context` package structure.

use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};

const LIB_TEMPLATE: &str = include_str!("../templates/new/lib.rs.tmpl");
const EMIT_TEMPLATE: &str = include_str!("../templates/new/emit.rs.tmpl");

/// Names derived from the user-supplied system name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldNames {
    /// `my_system` (Rust crate / module identifier, default directory name).
    pub snake: String,
    /// `my-system` (Cargo package name).
    pub kebab: String,
    /// `MySystem` (SysML package prefix).
    pub pascal: String,
}

/// JSON summary printed on stdout after a successful scaffold.
#[derive(Debug, Serialize)]
pub struct ScaffoldReport {
    pub ok: bool,
    pub package: String,
    pub crate_name: String,
    pub dir: String,
    pub sysml_packages: Vec<String>,
    pub files: Vec<String>,
    pub workspace_member: Option<String>,
    pub next_steps: Vec<String>,
}

/// Split `name` into lowercase words on `_`, `-`, spaces and camelCase boundaries.
pub fn derive_names(name: &str) -> Result<ScaffoldNames, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("system name must not be empty".into());
    }
    if let Some(bad) = trimmed
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ' ')))
    {
        return Err(format!(
            "invalid character `{bad}` in system name `{trimmed}` (use ASCII letters, digits, `_` or `-`)"
        ));
    }
    let mut words: Vec<String> = Vec::new();
    for chunk in trimmed.split(['_', '-', ' ']).filter(|s| !s.is_empty()) {
        let mut cur = String::new();
        let chars: Vec<char> = chunk.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            let boundary = c.is_ascii_uppercase()
                && i > 0
                && (chars[i - 1].is_ascii_lowercase()
                    || chars[i - 1].is_ascii_digit()
                    || chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase()));
            if boundary && !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            cur.push(c.to_ascii_lowercase());
        }
        if !cur.is_empty() {
            words.push(cur);
        }
    }
    if !words[0].starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Err(format!("system name `{trimmed}` must start with a letter"));
    }
    let snake = words.join("_");
    let kebab = words.join("-");
    let pascal = words
        .iter()
        .map(|w| {
            let mut cs = w.chars();
            match cs.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + cs.as_str(),
                None => String::new(),
            }
        })
        .collect();
    Ok(ScaffoldNames {
        snake,
        kebab,
        pascal,
    })
}

fn render(template: &str, names: &ScaffoldNames) -> String {
    template
        .replace("{{PASCAL}}", &names.pascal)
        .replace("{{SNAKE}}", &names.snake)
        .replace("{{KEBAB}}", &names.kebab)
}

fn cargo_toml_workspace(names: &ScaffoldNames) -> String {
    format!(
        r#"[package]
name = "{kebab}"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "{pascal} system model (scaffolded by `rsysdsl new`)"

[dependencies]
sysml-dsl = {{ workspace = true }}
sysml-emit = {{ workspace = true }}
"#,
        kebab = names.kebab,
        pascal = names.pascal
    )
}

fn cargo_toml_standalone(names: &ScaffoldNames, rsysdsl_root: &Path) -> String {
    let dep = |krate: &str| {
        rsysdsl_root
            .join("crates")
            .join(krate)
            .display()
            .to_string()
            .replace('\\', "/")
    };
    format!(
        r#"[package]
name = "{kebab}"
version = "0.1.0"
edition = "2021"
description = "{pascal} system model (scaffolded by `rsysdsl new --no-workspace`)"

# Standalone crate (not a member of the rsysdsl workspace). To join a workspace instead,
# delete this `[workspace]` table and add the crate path to that workspace's `members`.
[workspace]

[dependencies]
sysml-dsl = {{ path = "{dsl}" }}
sysml-emit = {{ path = "{emit}" }}
"#,
        kebab = names.kebab,
        pascal = names.pascal,
        dsl = dep("sysml-dsl"),
        emit = dep("sysml-emit"),
    )
}

/// Is `dir` an rsysdsl workspace root (root `Cargo.toml` with `[workspace]` + `crates/sysml-dsl`)?
fn is_rsysdsl_root(dir: &Path) -> bool {
    dir.join("crates/sysml-dsl/Cargo.toml").is_file()
        && fs::read_to_string(dir.join("Cargo.toml"))
            .map(|t| t.contains("[workspace]"))
            .unwrap_or(false)
}

/// Find the rsysdsl workspace root: walk up from the current directory, else fall back to the
/// workspace this binary was built from.
pub fn find_rsysdsl_root() -> Option<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        let mut cur: Option<&Path> = Some(cwd.as_path());
        while let Some(dir) = cur {
            if is_rsysdsl_root(dir) {
                return Some(dir.to_path_buf());
            }
            cur = dir.parent();
        }
    }
    let built = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if is_rsysdsl_root(&built) {
        return fs::canonicalize(&built).ok();
    }
    None
}

/// Lexically normalize an absolute path (resolve `.`/`..` without touching the filesystem).
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("failed to read current directory: {e}"))?
            .join(path)
    };
    Ok(normalize(&abs))
}

/// Insert `member` into the `members = [...]` array of a workspace `Cargo.toml`, preserving
/// the existing formatting. Returns `Ok(None)` when the member is already listed.
pub fn add_workspace_member(toml: &str, member: &str) -> Result<Option<String>, String> {
    let quoted = format!("\"{member}\"");
    let ws_start = toml
        .find("[workspace]")
        .ok_or("root Cargo.toml has no `[workspace]` table")?;
    let members_rel = toml[ws_start..]
        .find("members")
        .ok_or("root Cargo.toml `[workspace]` has no `members` array")?;
    let members_at = ws_start + members_rel;
    let open = members_at
        + toml[members_at..]
            .find('[')
            .ok_or("malformed `members` array (missing `[`)")?;
    let close = open
        + toml[open..]
            .find(']')
            .ok_or("malformed `members` array (missing `]`)")?;
    let body = &toml[open + 1..close];
    if body.contains(&quoted) {
        return Ok(None);
    }

    let mut out = String::with_capacity(toml.len() + member.len() + 16);
    if body.contains('\n') {
        // Multi-line array: append a new line using the indentation of the last entry.
        let entries_end = open + 1 + body.trim_end().len();
        let last_line = body.trim_end().lines().last().unwrap_or("");
        let indent: String = last_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        let indent = if indent.is_empty() {
            "    ".to_string()
        } else {
            indent
        };
        let needs_comma = !body.trim().is_empty() && !body.trim_end().ends_with(',');
        out.push_str(&toml[..entries_end]);
        if needs_comma {
            out.push(',');
        }
        out.push('\n');
        out.push_str(&indent);
        out.push_str(&quoted);
        out.push(',');
        out.push_str(&toml[entries_end..]);
    } else {
        // Single-line array: `members = ["a", "b"]`.
        let trimmed = body.trim_end();
        let entries_end = open + 1 + trimmed.len();
        out.push_str(&toml[..entries_end]);
        if trimmed.trim().is_empty() {
            out.push_str(&quoted);
        } else if trimmed.ends_with(',') {
            out.push(' ');
            out.push_str(&quoted);
        } else {
            out.push_str(", ");
            out.push_str(&quoted);
        }
        out.push_str(&toml[entries_end..]);
    }
    Ok(Some(out))
}

/// Generate the crate. `dir` overrides the default `<root>/examples/<snake_name>`.
pub fn scaffold(
    name: &str,
    dir: Option<PathBuf>,
    no_workspace: bool,
) -> Result<ScaffoldReport, String> {
    let names = derive_names(name)?;
    let root = find_rsysdsl_root().ok_or(
        "could not locate the rsysdsl workspace (run `rsysdsl new` from inside the rsysdsl checkout)",
    )?;
    let target = match dir {
        Some(d) => absolute(&d)?,
        None => root.join("examples").join(&names.snake),
    };
    if target.exists() {
        return Err(format!(
            "refusing to overwrite existing path '{}'",
            target.display()
        ));
    }

    // Validate workspace registration before writing anything.
    let mut new_root_toml: Option<(PathBuf, String, String)> = None;
    if !no_workspace {
        let rel = target.strip_prefix(&root).map_err(|_| {
            format!(
                "'{}' is outside the rsysdsl workspace '{}'; pass `--no-workspace` for a standalone crate",
                target.display(),
                root.display()
            )
        })?;
        let member = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let toml_path = root.join("Cargo.toml");
        let toml = fs::read_to_string(&toml_path)
            .map_err(|e| format!("failed to read '{}': {e}", toml_path.display()))?;
        if let Some(updated) = add_workspace_member(&toml, &member)? {
            new_root_toml = Some((toml_path, updated, member));
        }
    }

    let cargo_toml = if no_workspace {
        cargo_toml_standalone(&names, &root)
    } else {
        cargo_toml_workspace(&names)
    };
    let files: Vec<(PathBuf, String)> = vec![
        (target.join("Cargo.toml"), cargo_toml),
        (target.join("src/lib.rs"), render(LIB_TEMPLATE, &names)),
        (
            target.join("src/bin/emit.rs"),
            render(EMIT_TEMPLATE, &names),
        ),
    ];
    for (path, content) in &files {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create '{}': {e}", parent.display()))?;
        }
        fs::write(path, content)
            .map_err(|e| format!("failed to write '{}': {e}", path.display()))?;
    }

    let workspace_member = match new_root_toml {
        Some((path, updated, member)) => {
            fs::write(&path, updated)
                .map_err(|e| format!("failed to update '{}': {e}", path.display()))?;
            Some(member)
        }
        None => None,
    };

    let (test_cmd, emit_cmd, out) = if no_workspace {
        let manifest = target.join("Cargo.toml").display().to_string();
        let out = target
            .join(format!("target/{}.sysml", names.snake))
            .display()
            .to_string();
        (
            format!("cargo test --manifest-path {manifest}"),
            format!("cargo run -q --manifest-path {manifest} --bin emit -- -o {out}"),
            out,
        )
    } else {
        let out = format!("target/{}.sysml", names.snake);
        (
            format!("cargo test -p {}", names.kebab),
            format!("cargo run -q -p {} --bin emit -- -o {out}", names.kebab),
            out,
        )
    };

    Ok(ScaffoldReport {
        ok: true,
        package: names.kebab.clone(),
        crate_name: names.snake.clone(),
        dir: target.display().to_string(),
        sysml_packages: ["Types", "Requirements", "Subsystems", "Context"]
            .iter()
            .map(|l| format!("{}{l}", names.pascal))
            .collect(),
        files: files.iter().map(|(p, _)| p.display().to_string()).collect(),
        workspace_member,
        next_steps: vec![
            test_cmd,
            emit_cmd,
            format!("rsysdsl audit --files {out} --stdlib --strict --pretty"),
            format!("rsysdsl lint --files {out} --strict --format markdown"),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_names_from_any_case() {
        for input in [
            "my_system",
            "my-system",
            "MySystem",
            "mySystem",
            "My System",
        ] {
            let n = derive_names(input).unwrap();
            assert_eq!(n.snake, "my_system", "{input}");
            assert_eq!(n.kebab, "my-system", "{input}");
            assert_eq!(n.pascal, "MySystem", "{input}");
        }
        assert_eq!(derive_names("HVACUnit2").unwrap().snake, "hvac_unit2");
        assert!(derive_names("2fast").is_err());
        assert!(derive_names("bad/name").is_err());
        assert!(derive_names("").is_err());
    }

    #[test]
    fn adds_member_to_multiline_array_preserving_format() {
        let toml = "[workspace]\nresolver = \"2\"\nmembers = [\n    \"crates/a\",\n    \"examples/b\",\n]\nexclude = [\n    \"x\",\n]\n";
        let out = add_workspace_member(toml, "examples/new_one")
            .unwrap()
            .unwrap();
        assert_eq!(
            out,
            "[workspace]\nresolver = \"2\"\nmembers = [\n    \"crates/a\",\n    \"examples/b\",\n    \"examples/new_one\",\n]\nexclude = [\n    \"x\",\n]\n"
        );
        assert!(add_workspace_member(&out, "examples/new_one")
            .unwrap()
            .is_none());

        let no_trailing = "[workspace]\nmembers = [\n  \"a\"\n]\n";
        assert_eq!(
            add_workspace_member(no_trailing, "b").unwrap().unwrap(),
            "[workspace]\nmembers = [\n  \"a\",\n  \"b\",\n]\n"
        );
    }

    #[test]
    fn adds_member_to_single_line_array() {
        let toml = "[workspace]\nmembers = [\"a\", \"b\"]\n";
        assert_eq!(
            add_workspace_member(toml, "c").unwrap().unwrap(),
            "[workspace]\nmembers = [\"a\", \"b\", \"c\"]\n"
        );
        assert_eq!(
            add_workspace_member("[workspace]\nmembers = []\n", "c")
                .unwrap()
                .unwrap(),
            "[workspace]\nmembers = [\"c\"]\n"
        );
    }
}
