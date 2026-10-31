use std::{fs, process::Command};

mod common;
use common::success;

const TEXT: &str = include_str!("../examples/relation_contracts/text.loom");
const RANK: &str = include_str!("../examples/relation_contracts/rank.loom");
const MAIN: &str = include_str!("../examples/relation_contracts/main.loom");

#[test]
fn source_relation_contracts_preserve_keys_ranges_and_versions() {
    let root = tempfile::tempdir().unwrap();
    for (name, source) in [
        ("text.loom", TEXT),
        ("rank.loom", RANK),
        ("main.loom", MAIN),
    ] {
        fs::write(root.path().join(name), source).unwrap();
    }
    let cache = root.path().join("cache");
    let check = || {
        common::loom(&[
            "check",
            root.path().to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ])
    };
    success(&check());
    success(&check());
    for mode in ["test", "run"] {
        success(&common::loom(&[mode, root.path().to_str().unwrap()]));
    }
    let artifact = common::executable(root.path(), "relations");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", root.path().to_str().unwrap()])
                .arg("--frontend-cache")
                .arg(&cache)
                .arg("--output")
                .arg(&artifact)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        success(
            &Command::new(&artifact)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
    }
    for changed in [
        RANK.replace("first < last", "first <= last"),
        RANK.replace(
            "ensures rank_related(values[first], values[last])",
            "ensures values[first].minor < values[last].minor",
        ),
        RANK.replace(
            "if !rank_related(values[index - 1], values[index])",
            "if !rank_related(values[index], values[index - 1])",
        ),
        RANK.replace(
            "ensures rank_related(values[first], values[last]) {\n    true\n}",
            "ensures rank_related(values[first], values[last]) {\n    let alias = values\n    alias[first] = alias[last]\n    true\n}",
        ),
    ] {
        assert_ne!(changed, RANK);
        fs::write(root.path().join("rank.loom"), changed).unwrap();
        let output = check();
        assert!(!output.status.success(), "invalid relation contract accepted");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("proof") || error.contains("proved"), "{error}");
    }
    fs::write(root.path().join("rank.loom"), RANK).unwrap();
    success(&check());
}

#[test]
fn adjacent_checks_do_not_make_a_nontransitive_relation_transitive() {
    let root = tempfile::tempdir().unwrap();
    for (ty, expression) in [("Int", "left != right"), ("Float", "!(left > right)")] {
        fs::write(
            root.path().join("main.loom"),
            format!(
                r#"import std.list.length

fn related(left {ty}, right {ty}) Bool {{
    {expression}
}}

fn adjacent(values List[{ty}]) Bool {{
    var index = 1
    while index < length(values) {{
        if !related(values[index - 1], values[index]) {{
            return false
        }}
        index = index + 1
    }}
    true
}}

pub fn unsound(values List[{ty}], first Int, last Int) Bool
requires first >= 0 && first < last && last < length(values)
requires adjacent(values)
ensures related(values[first], values[last]) {{
    true
}}

fn main() {{}}
"#
            ),
        )
        .unwrap();
        let output = common::loom(&["check", root.path().to_str().unwrap()]);
        assert!(
            !output.status.success(),
            "unproved {ty} transitivity accepted"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proof") || error.contains("proved"),
            "{error}"
        );
    }
}
