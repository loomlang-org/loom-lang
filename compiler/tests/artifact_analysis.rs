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
fn reproduced_analysis_rejects_mixed_bases_and_keeps_frozen_source_queries() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let subject = root.join("subject 雪");
    fs::create_dir(&subject).unwrap();
    fs::write(
        subject.join("loom.toml"),
        "[module]\nname=\"artifact_analysis_subject\"\n",
    )
    .unwrap();
    let program = "fn main() {}\n";
    fs::write(subject.join("main.loom"), program).unwrap();
    let compiler = common::compiler();
    let native = Path::new(env!("CARGO_BIN_EXE_loom-native"));
    let compiler_hash = digest(&compiler);
    let native_hash = digest(native);
    let standard = common::root().join("compiler/std");
    for level in ["0", "2"] {
        let inspector = common::executable(&root, &format!("artifact-analysis-o{level}"));
        success(
            &common::command(&["build", "tools/deployment/artifact_analysis_trial"])
                .arg("--output")
                .arg(&inspector)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        let artifact = common::executable(&root, &format!("subject-o{level}"));
        let receipt = root.join(format!("subject-o{level}.receipt"));
        let mut receipts = Vec::new();
        let mut hashes = Vec::new();
        for mode in ["a", "b"] {
            success(
                &common::command(&["build"])
                    .arg(&subject)
                    .arg("--output")
                    .arg(&artifact)
                    .arg("--receipt")
                    .arg(&receipt)
                    .args(["--build-option", &format!("mode={mode}")])
                    .env("LOOM_OPT_LEVEL", level)
                    .output()
                    .unwrap(),
            );
            receipts.push(fs::read_to_string(&receipt).unwrap());
            hashes.push(digest(&artifact));
        }
        assert_eq!(hashes[0], hashes[1]);
        assert_eq!(
            field(&receipts[0], "checked-sha256"),
            field(&receipts[1], "checked-sha256")
        );
        assert_ne!(
            field(&receipts[0], "basis-sha256"),
            field(&receipts[1], "basis-sha256")
        );
        fs::write(&receipt, &receipts[0]).unwrap();
        let invoke = |label: &str, claim: &Path, analysis_mode: &str, scenario: &str| {
            Command::new(&inspector)
                .args([
                    compiler.as_os_str(),
                    compiler_hash.as_ref(),
                    native.as_os_str(),
                    native_hash.as_ref(),
                    subject.as_os_str(),
                    standard.as_os_str(),
                    claim.as_os_str(),
                    root.join(format!("{label}-o{level}")).as_os_str(),
                    level.as_ref(),
                    "a".as_ref(),
                    analysis_mode.as_ref(),
                    scenario.as_ref(),
                ])
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap()
        };
        let good = invoke("genuine", &receipt, "a", "mutate");
        success(&good);
        assert_eq!(String::from_utf8(good.stdout).unwrap(), "analysis bound\n");
        assert_eq!(
            fs::read_to_string(subject.join("main.loom")).unwrap(),
            program
        );
        assert_eq!(fs::read_to_string(&receipt).unwrap(), receipts[0]);
        assert_eq!(digest(&artifact), hashes[0]);
        let rejected = invoke("wrong-basis", &receipt, "b", "query");
        assert!(!rejected.status.success());
        assert!(
            String::from_utf8(rejected.stderr)
                .unwrap()
                .contains("analysis build basis differs")
        );
        assert!(!root.join(format!("wrong-basis-o{level}")).exists());
        let forged = root.join(format!("wrong-native-o{level}.receipt"));
        fs::write(
            &forged,
            receipts[0].replace(
                &format!("checked-sha256 {}", field(&receipts[0], "checked-sha256")),
                &format!("checked-sha256 {}", "f".repeat(64)),
            ),
        )
        .unwrap();
        let rejected = invoke("wrong-native", &forged, "a", "query");
        assert!(!rejected.status.success());
        assert!(
            String::from_utf8(rejected.stderr)
                .unwrap()
                .contains("analysis native input differs")
        );
        assert!(!root.join(format!("wrong-native-o{level}")).exists());
        let legacy = root.join(format!("legacy-o{level}.receipt"));
        fs::write(
            &legacy,
            receipts[0]
                .lines()
                .filter(|line| !line.starts_with("basis-sha256 "))
                .collect::<Vec<_>>()
                .join("\n")
                .replacen("loom-build-receipt 5", "loom-build-receipt 4", 1)
                + "\n",
        )
        .unwrap();
        let rejected = invoke("legacy", &legacy, "a", "query");
        assert!(!rejected.status.success());
        assert!(
            String::from_utf8(rejected.stderr)
                .unwrap()
                .contains("requires a v5 build basis")
        );
        assert!(!root.join(format!("legacy-o{level}")).exists());
    }
    let private = root.join("private-state");
    fs::create_dir(&private).unwrap();
    fs::write(
        private.join("loom.toml"),
        format!(
            "[module]\nname=\"private_artifact_analysis\"\n[dependencies.deployment]\npath={:?}\n",
            common::root().join("tools/deployment").to_str().unwrap()
        ),
    )
    .unwrap();
    fs::write(
        private.join("main.loom"),
        "import deployment.artifact_analysis.ArtifactAnalysis\nimport deployment.artifact_analysis.artifact_analysis_snapshot\npub fn inspect(value ArtifactAnalysis) { discard artifact_analysis_snapshot(value) }\nfn main() {}\n",
    )
    .unwrap();
    success(&common::command(&["check"]).arg(&private).output().unwrap());
    for access in ["value.state.analysis", "value.state.evidence"] {
        fs::write(private.join("main.loom"), format!("import deployment.artifact_analysis.ArtifactAnalysis\npub fn expose(value ArtifactAnalysis) {{ discard {access} }}\nfn main() {{}}\n")).unwrap();
        let rejected = common::command(&["check"]).arg(&private).output().unwrap();
        assert!(
            !rejected.status.success(),
            "artifact analysis private evidence was exposed"
        );
    }
}
