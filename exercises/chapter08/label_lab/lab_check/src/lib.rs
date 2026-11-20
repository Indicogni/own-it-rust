//! lab_check: tests for the SHAPE of the workspace.
//!
//! The other tests in this lab check what the code does. Some steps are about
//! where code lives and what the manifests say, and behaviour cannot show
//! that. The tests below read the files and check them directly.
//!
//! You do not need to edit this crate. Run it with `cargo test -p lab_check`.
//! Tests named `stepN_...` belong to step N of the lab.

use std::path::{Path, PathBuf};

/// The lab's root directory: the parent of this package.
pub fn lab_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("lab_check lives inside the lab directory")
        .to_path_buf()
}

/// Reads a file below the lab root, or explains which file is missing.
pub fn read(relative: &str) -> String {
    let path = lab_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// A manifest in a form that is easy to search: comments and all whitespace
/// removed from every line, single quotes turned into double quotes, empty
/// lines dropped. `version = "0.3.0"  # note` becomes `version="0.3.0"`.
pub fn compact(manifest: &str) -> Vec<String> {
    manifest
        .lines()
        .map(|line| {
            let code = line.split('#').next().unwrap_or("");
            code.split_whitespace()
                .collect::<String>()
                .replace('\'', "\"")
        })
        .filter(|line| !line.is_empty())
        .collect()
}

/// The lines of one table, for example `table(&lines, "workspace.package")`
/// for everything between `[workspace.package]` and the next `[...]` header.
pub fn table<'a>(lines: &'a [String], name: &str) -> Option<&'a [String]> {
    let header = format!("[{name}]");
    let start = lines.iter().position(|line| *line == header)? + 1;
    let len = lines[start..]
        .iter()
        .position(|line| line.starts_with('['))
        .unwrap_or(lines.len() - start);
    Some(&lines[start..start + len])
}

/// True when the `[package]` table inherits `key` from the workspace, in
/// either spelling: `key.workspace = true` or `key = { workspace = true }`.
pub fn inherits(lines: &[String], key: &str) -> bool {
    table(lines, "package").is_some_and(|package| {
        package.iter().any(|line| {
            *line == format!("{key}.workspace=true") || *line == format!("{key}={{workspace=true}}")
        })
    })
}

/// Rust source with `//` comments removed and whitespace collapsed, so that
/// commented-out code and formatting do not confuse a text search.
pub fn code_only(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split("//").next().unwrap_or("").trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMBERS: [&str; 3] = ["label_core", "label_slug", "label_cli"];

    fn manifest(member: &str) -> Vec<String> {
        compact(&read(&format!("{member}/Cargo.toml")))
    }

    #[test]
    fn step2_modules_live_in_their_own_files() {
        let lib = code_only(&read("label_core/src/lib.rs"));
        for module in ["validate", "model"] {
            let file = lab_root().join(format!("label_core/src/{module}.rs"));
            let directory = lab_root().join(format!("label_core/src/{module}/mod.rs"));
            assert!(
                file.is_file() || directory.is_file(),
                "move the contents of `mod {module}` into label_core/src/{module}.rs"
            );
            assert!(
                lib.lines().any(|line| {
                    line == format!("mod {module};") || line == format!("pub(crate) mod {module};")
                }),
                "lib.rs must declare the module with `mod {module};`: \
                 no body, and no plain `pub`, so that other crates cannot name it"
            );
        }
    }

    #[test]
    fn step3_tag_has_a_documented_example() {
        let source = read("label_core/src/lib.rs");
        let lines: Vec<&str> = source.lines().map(str::trim).collect();
        let at = lines
            .iter()
            .position(|line| line.starts_with("pub fn tag("))
            .expect("label_core/src/lib.rs must still define `pub fn tag(`");
        let docs: Vec<&str> = lines[..at]
            .iter()
            .rev()
            .take_while(|line| line.starts_with("///") || line.starts_with("#["))
            .copied()
            .collect();
        let fences = docs
            .iter()
            .filter(|line| line.starts_with("/// ```"))
            .count();
        assert!(
            fences >= 2,
            "the doc comment of `tag` needs an example between two ``` lines"
        );
        assert!(
            docs.iter().any(|line| line.contains("assert")),
            "the example on `tag` should assert something, so that it can fail"
        );
    }

    #[test]
    fn step4_cargo_run_has_a_default_binary() {
        let cli = manifest("label_cli");
        let package = table(&cli, "package").expect("label_cli needs a [package] table");
        assert!(
            package
                .iter()
                .any(|line| line == "default-run=\"label_cli\""),
            "label_cli/Cargo.toml: add default-run = \"label_cli\" to [package], \
             so that `cargo run -p label_cli` knows which of the two binaries to start"
        );
    }

    #[test]
    fn step5_shared_keys_come_from_the_workspace() {
        let root = compact(&read("Cargo.toml"));
        let shared = table(&root, "workspace.package")
            .expect("the root Cargo.toml needs a [workspace.package] table");
        for wanted in [
            "version=\"0.3.0\"",
            "edition=\"2024\"",
            "rust-version=\"1.99\"",
            "license=\"MIT\"",
        ] {
            assert!(
                shared.iter().any(|line| line == wanted),
                "[workspace.package] must contain {wanted}"
            );
        }
        for member in MEMBERS {
            let lines = manifest(member);
            for key in ["version", "edition", "rust-version", "license"] {
                assert!(
                    inherits(&lines, key),
                    "{member}/Cargo.toml must inherit `{key}`: write `{key}.workspace = true` \
                     in [package] instead of repeating the value"
                );
            }
        }
    }

    #[test]
    fn step5_path_dependencies_are_declared_once() {
        let root = compact(&read("Cargo.toml"));
        let shared = table(&root, "workspace.dependencies")
            .expect("the root Cargo.toml needs a [workspace.dependencies] table");
        for name in ["label_core", "label_slug"] {
            let entry = shared
                .iter()
                .find(|line| line.starts_with(&format!("{name}=")))
                .unwrap_or_else(|| panic!("[workspace.dependencies] must declare {name}"));
            assert!(
                entry.contains(&format!("path=\"{name}\"")) && entry.contains("version=\"0.3"),
                "declare {name} as {{ path = \"{name}\", version = \"0.3\" }}; found `{entry}`"
            );
        }
        let cli = manifest("label_cli");
        let dependencies =
            table(&cli, "dependencies").expect("label_cli needs a [dependencies] table");
        assert!(
            dependencies.iter().any(|line| {
                line == "label_core.workspace=true"
                    || line.starts_with("label_core={workspace=true")
            }),
            "label_cli must take label_core from the workspace: `label_core.workspace = true`"
        );
    }

    #[test]
    fn step5_lints_are_shared() {
        let root = compact(&read("Cargo.toml"));
        let forbids_unsafe = table(&root, "workspace.lints.rust")
            .is_some_and(|lints| lints.iter().any(|line| line == "unsafe_code=\"forbid\""))
            || table(&root, "workspace.lints").is_some_and(|lints| {
                lints
                    .iter()
                    .any(|line| line == "rust.unsafe_code=\"forbid\"")
            });
        assert!(
            forbids_unsafe,
            "the root Cargo.toml needs [workspace.lints.rust] with unsafe_code = \"forbid\""
        );
        for member in MEMBERS {
            let lines = manifest(member);
            let opted_in = table(&lines, "lints")
                .is_some_and(|lints| lints.iter().any(|line| line == "workspace=true"))
                || lines.iter().any(|line| line == "lints.workspace=true");
            assert!(
                opted_in,
                "{member}/Cargo.toml must opt in to the shared lints: \
                 add a [lints] table with `workspace = true`"
            );
        }
    }

    #[test]
    fn step6_slug_is_an_optional_dependency_behind_a_feature() {
        let core = manifest("label_core");
        let dependencies =
            table(&core, "dependencies").expect("label_core needs a [dependencies] table");
        let entry = dependencies
            .iter()
            .find(|line| line.starts_with("label_slug"))
            .expect("label_core must list label_slug under [dependencies]");
        assert!(
            entry.contains("optional=true"),
            "label_slug must be optional, so that a default build does not compile it; \
             found `{entry}`"
        );
        let features = table(&core, "features").expect("label_core needs a [features] table");
        assert!(
            features
                .iter()
                .any(|line| line == "slug=[\"dep:label_slug\"]"),
            "the feature must switch the dependency on: slug = [\"dep:label_slug\"]"
        );
    }

    #[test]
    fn step6_cli_forwards_the_feature() {
        let cli = manifest("label_cli");
        let features = table(&cli, "features").expect("label_cli needs a [features] table");
        assert!(
            features
                .iter()
                .any(|line| line == "slug=[\"label_core/slug\"]"),
            "label_cli's feature must enable the library's: slug = [\"label_core/slug\"]"
        );
    }

    #[test]
    fn step7_value_travels_through_the_build_script() {
        let script = code_only(&read("label_core/build.rs"));
        assert!(
            script.contains("rustc-env=LABEL_CORE_OPT_LEVEL="),
            "build.rs must print a cargo::rustc-env=LABEL_CORE_OPT_LEVEL=... line"
        );
        assert!(
            script.contains("OPT_LEVEL\")"),
            "build.rs must read OPT_LEVEL from its environment, not guess the value"
        );
        let lib = code_only(&read("label_core/src/lib.rs"));
        assert!(
            lib.contains("env!(\"LABEL_CORE_OPT_LEVEL\")"),
            "opt_level() must read the variable at compile time with env!"
        );
    }

    #[test]
    fn helpers_understand_a_small_manifest() {
        let lines = compact(
            "[package]\nname = 'demo'   # a comment\nedition.workspace = true\n\n\
             [dependencies]\nleft = { workspace = true }\n",
        );
        assert_eq!(lines[1], "name=\"demo\"");
        assert!(inherits(&lines, "edition"));
        assert!(!inherits(&lines, "version"));
        assert_eq!(
            table(&lines, "dependencies"),
            Some(&["left={workspace=true}".to_string()][..])
        );
        assert_eq!(table(&lines, "features"), None);
        assert_eq!(code_only("mod a; // note\n// mod b;\n"), "mod a;");
    }
}
