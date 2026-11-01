use std::{fs, process::Command};

mod common;
use common::success;

#[test]
fn library_sort_preserves_entry_counts_through_unknown_effectful_comparisons() {
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    let program = include_str!("../examples/sort_permutation/main.loom");
    let cache = package.path().join("cache");
    let check = || {
        common::loom(&[
            "check",
            package.path().to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ])
    };
    fs::write(&source, program).unwrap();
    success(&check());
    success(&check());
    for replacement in [
        "let changed = std.list.clone(values)\n    if length(changed) > 0 { changed[0] = 0 }\n    changed",
        "discard sorted(values, less)\n    values",
    ] {
        let changed = program.replace("    sorted(values, less)", &format!("    {replacement}"));
        assert_ne!(changed, program);
        fs::write(&source, format!("import std.list.clone\n{changed}")).unwrap();
        let output = check();
        assert!(!output.status.success(), "invalid permutation was proved");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proof") || error.contains("proved"),
            "{error}"
        );
    }
    fs::write(&source, program).unwrap();
    success(&check());
    for mode in ["test", "run"] {
        success(&common::loom(&[mode, package.path().to_str().unwrap()]));
    }
    let artifact = common::executable(package.path(), "sort-permutation");
    let ir = package.path().join("sort-permutation.ll");
    let sorting = include_str!("../std/list/sorting.loom");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", package.path().to_str().unwrap()])
                .arg("--output")
                .arg(&artifact)
                .arg("--emit-ir")
                .arg(&ir)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        let lowered = fs::read_to_string(&ir).unwrap();
        for (line, text) in sorting.lines().enumerate() {
            if text.trim_start().starts_with("assert ") {
                let column = text.len() - text.trim_start().len() + 1;
                assert!(
                    !lowered.contains(&format!(":{}:{column}: assertion failed", line + 1)),
                    "retained sort invariant at O{level}: {text}"
                );
            }
        }
        success(
            &Command::new(&artifact)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
    }
}
