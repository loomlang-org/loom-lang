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
    for body in ["[]", "{\n        discard less(1, 2)\n        values\n    }"] {
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
