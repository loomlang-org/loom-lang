use std::{fs, process::Command};

mod common;
use common::success;

#[test]
fn recursive_factories_seal_origins_without_inventing_values_or_hiding_aliases() {
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    let program = include_str!("../examples/recursive_factories/main.loom");
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
    let function = program
        .split("fn fresh_values")
        .nth(1)
        .unwrap()
        .split("fn make")
        .next()
        .unwrap();
    let swapped = "(seed List[Int], count Int, value Int) List[Int] {\n    choose([value, 2, 3], seed, false)\n}\n\nfn choose(first List[Int], second List[Int], flag Bool) List[Int] {\n    if flag {\n        return first\n    }\n    choose(second, first, true)\n}\n\n";
    for changed in [
        program.replace("return [value, 2, 3]", "return seed"),
        program.replace(function, swapped),
    ] {
        assert_ne!(changed, program);
        fs::write(&source, changed).unwrap();
        let output = check();
        assert!(
            !output.status.success(),
            "incoming alias was certified fresh"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("fresh unpublished storage"), "{error}");
    }
    fs::write(&source, program).unwrap();
    success(&check());
    for mode in ["test", "run"] {
        success(&common::loom(&[mode, package.path().to_str().unwrap()]));
    }
    let artifact = common::executable(package.path(), "recursive-factories");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", package.path().to_str().unwrap()])
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
}
