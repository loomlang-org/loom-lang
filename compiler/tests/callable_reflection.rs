use std::{fs, process::Command};
mod common;
use common::success;

const SOURCE: &str = include_str!("../examples/callable_reflection/main.loom");

#[test]
fn callable_inspection_rechecks_source_and_keeps_runtime_effects_out() {
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    let path = package.path().to_str().unwrap();
    let cache = package.path().join("cache");
    let check = || common::loom(&["check", path, "--frontend-cache", cache.to_str().unwrap()]);
    fs::write(&source, SOURCE).unwrap();
    success(&check());
    success(&check());
    let changed = SOURCE.replace(
        "    value + 1\n}",
        "    discard write_text(\"edited callback\")\n    value + 1\n}",
    );
    assert_ne!(changed, SOURCE);
    fs::write(&source, changed).unwrap();
    let rejected = check();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("postcondition"));
    fs::write(&source, SOURCE).unwrap();
    success(&check());
    for extra in [
        // Admitted operations do not establish an invocation's domain.
        r#"
fn divide(value Int) Int {
    1 / value
}
fn bad(value Int) Bool
requires comptime_callable(divide)
ensures divide(value) == divide(value) {
    true
}
"#,
        // An effect query does not manufacture a callback result guarantee.
        r#"
fn bad(value Int) Int
requires comptime_callable(increment)
ensures result > value {
    value
}
"#,
        // Unsupported effects produce false, not an assumed true capability.
        r#"
fn bad() Bool
ensures result {
    comptime_callable(noisy)
}
"#,
    ] {
        fs::write(&source, format!("{SOURCE}\n{extra}")).unwrap();
        let output = check();
        assert!(!output.status.success(), "unsound callable query: {extra}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proof") || error.contains("proved"),
            "{error}"
        );
        assert!(!error.contains("RuntimeFault"), "{error}");
    }
    fs::write(&source, SOURCE).unwrap();
    success(&check());
    let executable = common::executable(package.path(), "callables");
    let ir = package.path().join("callables.ll");
    for level in ["0", "2"] {
        success(
            &common::command(&[
                "build",
                path,
                "--output",
                executable.to_str().unwrap(),
                "--emit-ir",
                ir.to_str().unwrap(),
            ])
            .env("LOOM_OPT_LEVEL", level)
            .output()
            .unwrap(),
        );
        let output = Command::new(&executable)
            .env("LOOM_GC_STRESS", "1")
            .output()
            .unwrap();
        success(&output);
        assert!(output.stdout.is_empty(), "an inspected callback executed");
        assert!(
            !fs::read_to_string(&ir)
                .unwrap()
                .contains("callback must not execute")
        );
    }
}
