mod common;
use common::success;
use std::{fs, process::Command};

#[test]
fn checked_call_closures_keep_argument_cleanup_and_erased_call_obligations() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = fs::canonicalize(temporary.path()).unwrap();
    let inspector = common::executable(&directory, "inspect-calls");
    success(
        &common::command(&["build", "compiler/examples/call_closure"])
            .arg("--output")
            .arg(&inspector)
            .env("LOOM_OPT_LEVEL", "2")
            .output()
            .unwrap(),
    );
    let standard = common::root().join("compiler/std");
    let cases = [
        (
            "boundary",
            "fn boundary(value Int) { discard write_text(\"APPLICATION_EFFECT\") }\nfn main() { boundary(7) }",
            "boundary",
            false,
            false,
        ),
        (
            "argument",
            "fn argument() Int { discard write_text(\"APPLICATION_EFFECT\")\n7 }\nfn boundary(value Int) { discard write_text(\"APPLICATION_EFFECT\") }\nfn main() { boundary(argument()) }",
            "boundary",
            true,
            false,
        ),
        (
            "cleanup",
            "fn cleanup() { discard write_text(\"APPLICATION_EFFECT\") }\nfn boundary(value Int) { discard write_text(\"APPLICATION_EFFECT\") }\nfn main() { defer { cleanup() }\nboundary(7) }",
            "boundary",
            true,
            false,
        ),
        (
            "erased",
            "fn identity(value Int) Int { value }\nfn apply(callback fn(Int) Int) { discard callback(1) }\nfn main() { apply(identity) }",
            "",
            false,
            true,
        ),
    ];
    for (name, program, boundary, operations, issues) in cases {
        let package = directory.join(name);
        fs::create_dir(&package).unwrap();
        fs::write(
            package.join("loom.toml"),
            "[module]\nname = \"call_closure_subject\"\n",
        )
        .unwrap();
        fs::write(
            package.join("main.loom"),
            format!("import std.io.write_text\n{program}\n"),
        )
        .unwrap();
        let output = Command::new(&inspector)
            .args([package.as_os_str(), standard.as_os_str()])
            .arg(boundary)
            .env("LOOM_GC_STRESS", "1")
            .output()
            .unwrap();
        success(&output);
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.contains("APPLICATION_EFFECT"));
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 3, "{name}: {text}");
        let count = |index: usize, label: &str| -> usize {
            lines[index].strip_prefix(label).unwrap().parse().unwrap()
        };
        assert!(count(0, "calls ") > 0);
        assert_eq!(count(1, "operations ") > 0, operations, "{name}: {text}");
        assert_eq!(count(2, "issues ") > 0, issues, "{name}: {text}");
    }
}
