use std::{fs, process::Command};
mod common;
use common::success;

const PREDICATES: &str = include_str!("../examples/predicate_contracts/main.loom");

#[test]
fn immutable_predicate_evidence_survives_composition_and_rechecks_edits() {
    let package = tempfile::tempdir().unwrap();
    let path = package.path().to_str().unwrap();
    let source = package.path().join("main.loom");
    fs::write(&source, PREDICATES).unwrap();
    fs::write(
        package.path().join("lifetimes.loom"),
        include_str!("../examples/predicate_contracts/lifetimes.loom"),
    )
    .unwrap();
    fs::write(
        package.path().join("lists.loom"),
        include_str!("../examples/predicate_contracts/lists.loom"),
    )
    .unwrap();
    let cache = package.path().join("cache");
    let check = || common::loom(&["check", path, "--frontend-cache", cache.to_str().unwrap()]);
    success(&check());
    success(&check());
    for extra in [
        // Truth is not available without an executed precondition or call.
        "fn bad(value Text) Text\nensures balanced(Input.Plain(result))\n{\n    value\n}\n",
        // Even a tautology must establish successful evaluation first.
        "fn bad(value Text) Bool\nensures balanced(Input.Plain(value)) == balanced(Input.Plain(value))\n{\n    true\n}\n",
        "fn bad(value Text, other Text) Text\nrequires balanced(Input.Plain(value))\nensures balanced(Input.Plain(result))\n{\n    other\n}\n",
        "fn bad(value Text, skip Bool) Text\nrequires skip || balanced(Input.Plain(value))\nensures balanced(Input.Plain(result))\n{\n    value\n}\n",
        "fn bad(value Text) Text\nrequires balanced(Input.Plain(value))\nensures balanced(Input.Plain(result))\n{\n    concat(value, \"(\")\n}\n",
        "fn bad(value Text, count Int) Text\nrequires count >= 0 && balanced(Input.Plain(value))\nensures balanced(Input.Plain(result))\n{\n    var current = value\n    var index = 0\n    while index < count {\n        current = \"(\"\n        index = index + 1\n    }\n    current\n}\n",
        "fn bad(values List[Text]) Bool\nensures all_balanced(values) == all_balanced(values)\n{\n    true\n}\n",
        "fn bad(values List[Text]) Bool\nensures std.list.all(values, nonempty_balanced) == std.list.all(values, nonempty_balanced)\n{\n    true\n}\n",
        "fn bad(values List[Text]) List[Text]\nrequires all_balanced(values)\nensures all_balanced(result)\n{\n    push(values, \"(\")\n    values\n}\n",
    ] {
        fs::write(
            &source,
            format!("import std.text.concat\nimport std.list.all\n{PREDICATES}\n{extra}"),
        )
        .unwrap();
        let output = check();
        assert!(!output.status.success(), "unsound predicate proof: {extra}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proof") || error.contains("proved"),
            "{error}"
        );
    }
    fs::write(&source, PREDICATES).unwrap();
    success(&check());
    let artifact = common::executable(package.path(), "predicates");
    for level in ["0", "2"] {
        success(
            &common::command(&["test", path])
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        success(
            &common::command(&["build", path, "--output", artifact.to_str().unwrap()])
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
    success(&common::loom(&["run", path]));
}

#[test]
fn opaque_predicates_do_not_bypass_effect_or_mutable_input_checks() {
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    for (helper, contract) in [
        (
            r#"import std.io.write_text
fn noisy(value Text) Bool {
    discard write_text(value)
    true
}
"#,
            r#"fn bad(value Text) Text
requires noisy(value)
ensures noisy(result)
{
    value
}
"#,
        ),
        (
            r#"import std.list.length
fn changing(values List[Int]) Bool {
    var index = 0
    var total = 0
    while index < std.list.length(values) {
        total = total + values[index]
        index = index + 1
    }
    total > 0
}
"#,
            r#"fn bad(values List[Int]) List[Int]
requires changing(values)
ensures changing(result)
{
    values
}
"#,
        ),
    ] {
        fs::write(&source, helper).unwrap();
        success(&common::loom(&["check", package.path().to_str().unwrap()]));
        fs::write(&source, format!("{helper}\n{contract}")).unwrap();
        let output = common::loom(&["check", package.path().to_str().unwrap()]);
        assert!(
            !output.status.success(),
            "unsupported purity claim: {contract}"
        );
    }
}

#[test]
fn predicate_origins_do_not_equate_ieee_zeros_or_distinct_targets() {
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    let program = r#"
fn positive_reciprocal(value Float) Bool {
    var index = 0
    var found = false
    while index < 2 {
        found = 1.0 / value > 0.0
        index = index + 1
    }
    found
}

fn preserved(value Float) Float
requires positive_reciprocal(value)
ensures positive_reciprocal(result)
{
    value
}

fn main() {
    assert positive_reciprocal(preserved(0.0))
    assert !positive_reciprocal(-0.0)
}
"#;
    fs::write(&source, program).unwrap();
    let check = || common::loom(&["check", package.path().to_str().unwrap()]);
    success(&check());
    for extra in [
        "fn bad(a Float, b Float) Float\nrequires a == b && positive_reciprocal(a)\nensures positive_reciprocal(result)\n{\n    b\n}\n",
        "fn other(value Float) Bool {\n    !positive_reciprocal(value)\n}\nfn bad(value Float) Float\nrequires positive_reciprocal(value)\nensures other(result)\n{\n    value\n}\n",
    ] {
        fs::write(&source, format!("{program}\n{extra}")).unwrap();
        let output = check();
        assert!(
            !output.status.success(),
            "unsound origin substitution: {extra}"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proof") || error.contains("proved"),
            "{error}"
        );
    }
    fs::write(&source, program).unwrap();
    success(&common::loom(&["run", package.path().to_str().unwrap()]));
}
