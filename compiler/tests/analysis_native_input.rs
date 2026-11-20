mod common;
use common::success;
use sha2::{Digest, Sha256};
use std::{fs, process::Command};

#[test]
fn checked_analysis_matches_actual_backend_input_and_detects_same_text_ast_edits() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = fs::canonicalize(temporary.path()).unwrap();
    let inspector = common::executable(&directory, "inspect-input");
    success(
        &common::command(&["build", "compiler/examples/analysis_native_input"])
            .arg("--output")
            .arg(&inspector)
            .env("LOOM_OPT_LEVEL", "2")
            .output()
            .unwrap(),
    );
    let standard = common::root().join("compiler/std");
    let cases = [
        (
            "io",
            "import std.io.write_text\nfn main() { discard write_text(\"APPLICATION_EFFECT\\n\") }\n",
        ),
        (
            "generic",
            "fn identity[T](value T) T { value }\nfn main() { discard identity(7)\ndiscard identity(true) }\n",
        ),
        (
            "async",
            "concept Worker { async fn work(self Self) Int }\nimpl Worker for Int { async fn work(self Int) Int { self } }\nasync fn main() { let worker dyn Worker = 7\ndiscard worker.work().await }\n",
        ),
    ];
    for (name, program) in cases {
        let package = directory.join(name);
        fs::create_dir(&package).unwrap();
        fs::write(
            package.join("loom.toml"),
            "[module]\nname = \"native_input_subject\"\n",
        )
        .unwrap();
        let source = package.join("main.loom");
        fs::write(&source, program).unwrap();
        let artifact = common::executable(&directory, &format!("{name}-app"));
        let receipt = directory.join(format!("{name}.receipt"));
        success(
            &common::command(&["build"])
                .arg(&package)
                .arg("--output")
                .arg(&artifact)
                .arg("--receipt")
                .arg(&receipt)
                .env("LOOM_OPT_LEVEL", "2")
                .output()
                .unwrap(),
        );
        let receipt = fs::read_to_string(receipt).unwrap();
        let field = |prefix: &str| {
            receipt
                .lines()
                .find_map(|line| line.strip_prefix(prefix))
                .unwrap()
        };
        let artifact_hash = format!("{:x}", Sha256::digest(fs::read(artifact).unwrap()));
        assert_eq!(field("artifact-sha256 "), artifact_hash);
        let output = Command::new(&inspector)
            .args([package.as_os_str(), standard.as_os_str()])
            .env("LOOM_GC_STRESS", "1")
            .output()
            .unwrap();
        success(&output);
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.contains("APPLICATION_EFFECT"));
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{name}: {text}");
        let original = lines[0].strip_prefix("original ").unwrap();
        let edited = lines[1].strip_prefix("edited ").unwrap();
        assert_eq!(original, field("checked-sha256 "), "{name}: {text}");
        assert_ne!(edited, original, "{name}: {text}");
        assert_eq!(fs::read_to_string(source).unwrap(), program);
    }
}
