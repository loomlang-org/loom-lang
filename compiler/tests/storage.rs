mod common;
use common::success;
use std::fs;

#[test]
fn source_record_codecs_use_native_fields_and_checked_constructors() {
    let directory = tempfile::tempdir().unwrap();
    let executable = common::executable(directory.path(), "storage");
    let example = "compiler/examples/storage";
    success(&common::loom(&["check", example]));
    for level in ["0", "2"] {
        for (mode, package) in [
            ("test", "compiler/std/storage"),
            ("test", example),
            ("build", example),
        ] {
            let mut command = common::command(&[mode, package]);
            if mode == "test" {
                command.arg("--no-run");
            }
            success(
                &command
                    .arg("--output")
                    .arg(&executable)
                    .env("LOOM_OPT_LEVEL", level)
                    .output()
                    .unwrap(),
            );
            success(&common::run_tasks(&executable));
        }
    }
    success(&common::loom(&["run", example]));
}

#[test]
fn refined_record_codecs_recheck_changed_record_predicates() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("main.loom");
    let program = include_str!("../examples/storage/main.loom");
    let cache = directory.path().join("cache");
    let check = || {
        common::loom(&[
            "check",
            directory.path().to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ])
    };
    fs::write(&source, program).unwrap();
    success(&check());
    success(&check());

    let weakened = program.replace("self.id > 0 && self.id <= self.amount", "self.id > 0");
    assert_ne!(weakened, program);
    fs::write(&source, &weakened).unwrap();
    let rejected = check();
    assert!(!rejected.status.success());
    let message = String::from_utf8_lossy(&rejected.stderr);
    assert!(
        message.contains("proof") || message.contains("proved"),
        "{message}"
    );

    // The changed source is otherwise well typed: only its false guarantee
    // prevents checking. This control is checked, not executed.
    let control = weakened.replace("ensures result >= value.id {", "{");
    assert_ne!(control, weakened);
    fs::write(&source, control).unwrap();
    success(&check());

    fs::write(&source, program).unwrap();
    success(&check());
    success(&common::loom(&[
        "run",
        directory.path().to_str().unwrap(),
        "--frontend-cache",
        cache.to_str().unwrap(),
    ]));
}
