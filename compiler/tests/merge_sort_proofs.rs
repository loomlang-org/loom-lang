use std::{fs, process::Command};

mod common;
use common::success;

#[test]
fn recursive_sort_composes_entry_contracts_without_retaining_loop_scans() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("sort");
    let library = root.path().join("merge");
    fs::create_dir(&package).unwrap();
    fs::create_dir(&library).unwrap();
    let program = include_str!("../examples/merge_sort_contracts/main.loom");
    let merge = include_str!("../examples/merge_contracts/main.loom");
    fs::write(library.join("main.loom"), merge).unwrap();
    fs::write(
        library.join("loom.toml"),
        include_str!("../examples/merge_contracts/loom.toml"),
    )
    .unwrap();
    fs::write(
        package.join("loom.toml"),
        include_str!("../examples/merge_sort_contracts/loom.toml")
            .replace("../merge_contracts", "../merge"),
    )
    .unwrap();
    let source = package.join("main.loom");
    fs::write(&source, program).unwrap();
    let cache = root.path().join("cache");
    let check = || {
        common::loom(&[
            "check",
            package.to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ])
    };
    success(&check());
    success(&check());
    for mode in ["test", "run"] {
        success(&common::loom(&[
            mode,
            package.to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ]));
    }
    let artifact = common::executable(&package, "merge-sort");
    let ir = package.join("merge-sort.ll");
    for level in ["0", "2"] {
        success(
            &common::command(&[
                "build",
                package.to_str().unwrap(),
                "--frontend-cache",
                cache.to_str().unwrap(),
            ])
            .arg("--output")
            .arg(&artifact)
            .arg("--emit-ir")
            .arg(&ir)
            .env("LOOM_OPT_LEVEL", level)
            .output()
            .unwrap(),
        );
        let lowered = fs::read_to_string(&ir).unwrap();
        let mut invariants = 0;
        for text in [program, merge] {
            for (line, statement) in text.lines().enumerate() {
                if statement.starts_with("        assert ") {
                    invariants += 1;
                    assert!(
                        !lowered.contains(&format!(":{}:9: assertion failed", line + 1)),
                        "retained invariant at line {} (O{level})",
                        line + 1
                    );
                }
            }
        }
        assert_eq!(invariants, 11);
        success(
            &Command::new(&artifact)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
    }
    for changed in [
        program.replace("push(left, values[index])", "push(left, 0)"),
        program.replace(
            "index = index + 1\n    }\n    let first = sorted_merge(left)",
            "index = index + 2\n    }\n    let first = sorted_merge(left)",
        ),
    ] {
        assert_ne!(changed, program);
        fs::write(&source, changed).unwrap();
        let output = check();
        assert!(
            !output.status.success(),
            "incorrect recursive sort was accepted"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proof") || error.contains("proved"),
            "{error}"
        );
    }
    fs::write(&source, program).unwrap();
    success(&check());
}
