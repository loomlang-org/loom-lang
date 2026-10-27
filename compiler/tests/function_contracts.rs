use std::{fs, process::Command};
mod common;
use common::success;

const FUNCTIONS: &str = include_str!("../examples/function_contracts/main.loom");
const FLOATS: &str = include_str!("../examples/function_contracts/floats.loom");

#[test]
fn immutable_function_results_compose_and_reject_unestablished_evidence() {
    let package = tempfile::tempdir().unwrap();
    let path = package.path().to_str().unwrap();
    let source = package.path().join("main.loom");
    fs::write(&source, FUNCTIONS).unwrap();
    fs::write(package.path().join("floats.loom"), FLOATS).unwrap();
    let cache = package.path().join("cache");
    let check = || common::loom(&["check", path, "--frontend-cache", cache.to_str().unwrap()]);
    success(&check());
    success(&check());
    for extra in [
        // No call or entry predicate has established the function's domain.
        "fn bad(value Text) Bool\nensures count_open(value) == count_open(value)\n{\n    true\n}\n",
        "fn forever(value Text) Int {\n    forever(value)\n}\nfn bad(value Text) Bool\nensures forever(value) == forever(value)\n{\n    true\n}\n",
        // A successful call at another argument supplies no result equation.
        "fn bad(value Text, other Text) Int\nensures result == count_open(value)\n{\n    count_open(other)\n}\n",
        // Different result fields must not collapse to the same symbol.
        "fn bad(value Text) Bool\nrequires pair(value).0 > 0\nensures pair(value).0 == 0\n{\n    true\n}\n",
        // The same immutable result is reusable, but is not necessarily zero.
        "fn bad(value Text) Int\nensures result == 0\n{\n    count_open(value)\n}\n",
        // A returned Float may be NaN; successful evaluation is not IEEE equality.
        "fn bad(value Float) Float\nensures result == reciprocal(value)\n{\n    reciprocal(value)\n}\n",
        // IEEE numerical equality does not preserve a reciprocal's sign at zero.
        "fn bad(a Float, b Float) Float\nrequires a == b && reciprocal(a) > 0.0\nensures reciprocal(result) > 0.0\n{\n    b\n}\n",
        // Exact aggregate binding cannot establish unrelated output fields.
        "fn bad(value Text) Summary\nensures result.count == 0\n{\n    summarize(value)\n}\n",
        // Contract results retain the actual function target's identity.
        "fn different(value Text) Int {\n    count_open(value) + 1\n}\nfn bad(value Text) Int\nensures result == different(value)\n{\n    count_open(value)\n}\n",
    ] {
        fs::write(&source, format!("{FUNCTIONS}\n{extra}")).unwrap();
        let output = check();
        assert!(!output.status.success(), "unsound function proof: {extra}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proof") || error.contains("proved"),
            "{error}"
        );
        assert!(!error.contains("RuntimeFault"), "{error}");
    }
    fs::write(&source, FUNCTIONS).unwrap();
    success(&check());
    let artifact = common::executable(package.path(), "functions");
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
