use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};
mod common;
use common::success;

fn digest(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

fn field(receipt: &str, name: &str) -> String {
    receipt
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{name} ")))
        .unwrap()
        .to_owned()
}

#[test]
fn actual_reproduction_rejects_mixed_artifacts_and_changed_build_inputs() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let subject = root.join("subject 雪");
    fs::create_dir(&subject).unwrap();
    fs::write(
        subject.join("loom.toml"),
        "[module]\nname = \"reproduction_subject\"\n",
    )
    .unwrap();
    let native = Path::new(env!("CARGO_BIN_EXE_loom-native"));
    let compiler = common::compiler();
    let compiler_hash = digest(&compiler);
    let native_hash = digest(native);
    let standard = common::root().join("compiler/std");
    let option = "literal=a=b #? 雪";
    let marker = root.join("application-must-not-run");
    let program = |payload| {
        format!(
            "import std.file.write_text\nfn main() {{ discard write_text({:?}, {:?}) }}\n",
            marker.to_str().unwrap(),
            payload
        )
    };
    for level in ["0", "2"] {
        let trial = common::executable(&root, &format!("reproduction-trial-o{level}"));
        success(
            &common::command(&["build"])
                .arg(common::root().join("tools/deployment/reproduction_trial"))
                .args(["--output"])
                .arg(&trial)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        let mut artifacts = Vec::new();
        let mut receipts = Vec::new();
        for (name, payload) in [("a", "artifact-A"), ("b", "artifact-B")] {
            fs::write(subject.join("main.loom"), program(payload)).unwrap();
            let artifact = common::executable(&root, &format!("{name}-o{level}"));
            let receipt = root.join(format!("{name}-o{level}.receipt"));
            success(
                &common::command(&["build"])
                    .arg(&subject)
                    .arg("--output")
                    .arg(&artifact)
                    .arg("--receipt")
                    .arg(&receipt)
                    .args(["--build-option", &format!("mode={option}")])
                    .env("LOOM_OPT_LEVEL", level)
                    .output()
                    .unwrap(),
            );
            artifacts.push(artifact);
            receipts.push(receipt);
        }
        let a = fs::read_to_string(&receipts[0]).unwrap();
        let b = fs::read_to_string(&receipts[1]).unwrap();
        assert_ne!(digest(&artifacts[0]), digest(&artifacts[1]));
        let invoke_compiler = |selected_compiler: &Path,
                               label: &str,
                               receipt: &Path,
                               selected_hash: &str,
                               native_hash: &str,
                               mode: &str| {
            Command::new(&trial)
                .args([
                    selected_compiler.as_os_str(),
                    selected_hash.as_ref(),
                    native.as_os_str(),
                    native_hash.as_ref(),
                    subject.as_os_str(),
                    standard.as_os_str(),
                    receipt.as_os_str(),
                    root.join(format!("{label}-o{level}")).as_os_str(),
                    level.as_ref(),
                    mode.as_ref(),
                ])
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap()
        };
        let invoke =
            |label: &str, receipt: &Path, selected_hash: &str, native_hash: &str, mode: &str| {
                invoke_compiler(&compiler, label, receipt, selected_hash, native_hash, mode)
            };
        let good = invoke(
            "genuine",
            &receipts[1],
            &compiler_hash,
            &native_hash,
            option,
        );
        success(&good);
        assert_eq!(
            String::from_utf8(good.stdout).unwrap(),
            format!("reproduced {}\n", digest(&artifacts[1]))
        );
        assert_eq!(
            digest(&root.join(format!("genuine-o{level}/artifact"))),
            digest(&artifacts[1])
        );
        assert!(!marker.exists());
        let mixed = root.join(format!("mixed-o{level}.receipt"));
        fs::write(
            &mixed,
            b.replace(
                &format!("artifact-sha256 {}", field(&b, "artifact-sha256")),
                &format!("artifact-sha256 {}", field(&a, "artifact-sha256")),
            )
            .replace(
                &format!("artifact-path {}", field(&b, "artifact-path")),
                &format!("artifact-path {}", field(&a, "artifact-path")),
            ),
        )
        .unwrap();
        let rejected = invoke("mixed", &mixed, &compiler_hash, &native_hash, option);
        assert!(!rejected.status.success());
        assert!(
            String::from_utf8(rejected.stderr)
                .unwrap()
                .contains("identities differ")
        );
        assert_eq!(
            digest(&root.join(format!("mixed-o{level}/artifact"))),
            digest(&artifacts[1])
        );
        let changed = invoke(
            "changed-option",
            &receipts[1],
            &compiler_hash,
            &native_hash,
            "different",
        );
        assert!(!changed.status.success());
        let changed_receipt =
            fs::read_to_string(root.join(format!("changed-option-o{level}/build.receipt")))
                .unwrap();
        assert_eq!(
            field(&changed_receipt, "artifact-sha256"),
            field(&b, "artifact-sha256")
        );
        assert_ne!(
            field(&changed_receipt, "basis-sha256"),
            field(&b, "basis-sha256")
        );
        for (label, selected, backend) in [
            ("wrong-compiler", "0".repeat(64), native_hash.clone()),
            ("wrong-native", compiler_hash.clone(), "0".repeat(64)),
        ] {
            let failed = invoke(label, &receipts[1], &selected, &backend, option);
            assert!(!failed.status.success());
            assert!(!root.join(format!("{label}-o{level}")).exists());
        }
        let reused = root.join(format!("reused-o{level}"));
        fs::create_dir(&reused).unwrap();
        fs::write(reused.join("keep"), "existing output").unwrap();
        assert!(
            !invoke("reused", &receipts[1], &compiler_hash, &native_hash, option)
                .status
                .success()
        );
        assert_eq!(
            fs::read_to_string(reused.join("keep")).unwrap(),
            "existing output"
        );
        let legacy = root.join(format!("legacy-o{level}.receipt"));
        let legacy_text = b
            .replace("loom-build-receipt 5", "loom-build-receipt 4")
            .lines()
            .filter(|line| !line.starts_with("basis-sha256 "))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(&legacy, legacy_text).unwrap();
        let failed = invoke("legacy", &legacy, &compiler_hash, &native_hash, option);
        assert!(!failed.status.success());
        assert!(
            String::from_utf8(failed.stderr)
                .unwrap()
                .contains("v5 build basis")
        );
        assert!(!root.join(format!("legacy-o{level}")).exists());
        fs::write(subject.join("main.loom"), "fn main( {\n").unwrap();
        let failed = invoke(
            "compiler-error",
            &receipts[1],
            &compiler_hash,
            &native_hash,
            option,
        );
        assert!(!failed.status.success());
        assert!(
            String::from_utf8(failed.stderr)
                .unwrap()
                .contains("did not complete successfully")
        );
        fs::write(subject.join("main.loom"), program("artifact-B")).unwrap();
        let wrapper_source = root.join(format!("wrapper-source-o{level}"));
        fs::create_dir(&wrapper_source).unwrap();
        let changed_claim = b.replace(
            &format!("checked-sha256 {}", field(&b, "checked-sha256")),
            &format!("checked-sha256 {}", field(&a, "checked-sha256")),
        );
        let wrapper_program = r#"import std.process.arguments
import std.process.capture
import std.process.ExitStatus
import std.process.exit_code
import std.list.get
import std.list.length
import std.list.push
import std.file.write_text
import std.result.Result
fn main() {
    let args = arguments()
    let forwarded = [$COMPILER]
    var index = 1
    while index < length(args) {
        push(forwarded, get(args, index))
        index = index + 1
    }
    match capture(forwarded) {
        Result.Ok(output) => {
            match output.status {
                ExitStatus.Exited(code) => { if code != 0 { exit_code(1) } }
                ExitStatus.Terminated => { exit_code(1) }
            }
            discard write_text($RECEIPT, $TEXT)
        }
        Result.Err(_) => { exit_code(1) }
    }
}
"#
        .replace("$COMPILER", &format!("{:?}", compiler.to_str().unwrap()))
        .replace("$RECEIPT", &format!("{:?}", receipts[1].to_str().unwrap()))
        .replace("$TEXT", &format!("{changed_claim:?}"));
        fs::write(wrapper_source.join("main.loom"), wrapper_program).unwrap();
        let wrapper = common::executable(&root, &format!("compiler-wrapper-o{level}"));
        success(
            &common::command(&["build"])
                .arg(&wrapper_source)
                .arg("--output")
                .arg(&wrapper)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        let changed = invoke_compiler(
            &wrapper,
            "changed-claim",
            &receipts[1],
            &digest(&wrapper),
            &native_hash,
            option,
        );
        assert!(!changed.status.success());
        assert!(
            String::from_utf8(changed.stderr)
                .unwrap()
                .contains("claimed build receipt changed")
        );
        assert_eq!(fs::read_to_string(&receipts[1]).unwrap(), changed_claim);
        fs::write(&receipts[1], &b).unwrap();
        let original = fs::read(&artifacts[1]).unwrap();
        fs::write(&artifacts[1], b"changed artifact").unwrap();
        assert!(
            !invoke(
                "tampered",
                &receipts[1],
                &compiler_hash,
                &native_hash,
                option
            )
            .status
            .success()
        );
        assert!(!root.join(format!("tampered-o{level}")).exists());
        fs::write(&artifacts[1], original).unwrap();
        assert_eq!(fs::read_to_string(&receipts[1]).unwrap(), b);
        assert_eq!(digest(&artifacts[1]), field(&b, "artifact-sha256"));
        assert!(!marker.exists());
    }
}
