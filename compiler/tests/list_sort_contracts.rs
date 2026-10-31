use std::{fs, process::Command};

mod common;
use common::success;

#[test]
fn library_sort_shapes_recheck_callbacks_and_constrained_results() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("main.loom");
    let program = include_str!("../examples/list_sort_contracts/main.loom");
    fs::write(&source, program).unwrap();
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
        let selected_cache = if mode == "test" {
            root.path().join("test-cache")
        } else {
            cache.clone()
        };
        success(&common::loom(&[
            mode,
            root.path().to_str().unwrap(),
            "--frontend-cache",
            selected_cache.to_str().unwrap(),
        ]));
    }
    let artifact = common::executable(root.path(), "sort-shapes");
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
    for body in [
        "[]",
        r#"{
        if length(values) > 0 {
            let first = values[0]
            discard less(first, first)
        }
        values
    }"#,
    ] {
        let changed = program.replace("sorted(values, less)", body);
        assert_ne!(changed, program);
        fs::write(&source, changed).unwrap();
        let output = check();
        assert!(!output.status.success(), "invalid shape proof was accepted");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proof") || error.contains("proved"),
            "{error}"
        );
    }
    fs::write(&source, program).unwrap();
    success(&check());
}

#[test]
fn child_backedges_cannot_hide_callback_access_to_a_private_header() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("main.loom");
    fs::write(
        source,
        r#"import std.list.new
import std.list.push
import std.list.length

record Node {
    children List[Node]
}

pub fn exposed(visit fn(Node) Bool) List[Node]
ensures length(result) == 1 {
    let children = new[Node]()
    let child = Node { children = children }
    let output = new[Node]()
    push(output, child)
    var i = 0
    while i < 2 {
        if i == 1 {
            push(children, Node { children = output })
        }
        discard visit(child)
        i = i + 1
    }
    output
}

fn main() {}
"#,
    )
    .unwrap();
    let output = common::loom(&["check", root.path().to_str().unwrap()]);
    assert!(
        !output.status.success(),
        "exposed header retained its shape"
    );
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("postcondition"), "{error}");
}
