//! End-to-end CLI integration tests (`RISC0_DEV_MODE=1`, fast dev proofs).
//!
//! These exercise the real `quorum` binary the same way `scripts/demo.sh` does:
//! create → propose → aggregated approve-all → execute → rotate → activate keys.

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn quorum() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quorum"));
    command.env("RISC0_DEV_MODE", "1");
    command
}

fn run(dir: &Path, args: &[&str]) -> Output {
    quorum()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to spawn quorum CLI")
}

fn run_ok(dir: &Path, args: &[&str]) -> String {
    let out = run(dir, args);
    assert!(
        out.status.success(),
        "`quorum {}` failed\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn run_err(dir: &Path, args: &[&str]) -> String {
    let out = run(dir, args);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.status.success(),
        "`quorum {}` unexpectedly succeeded\n{combined}",
        args.join(" ")
    );
    combined
}

fn workdir(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "quorum-cli-it-{name}-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

const RECIPIENT: &str = "0909090909090909090909090909090909090909090909090909090909090909";

#[test]
fn full_flow_with_aggregated_approval() {
    let dir = workdir("aggregated");
    let tiers = r#"[{"id":1,"threshold":2,"max_amount":1000}]"#;

    let stdout = run_ok(
        &dir,
        &[
            "create",
            "--threshold",
            "2",
            "--members",
            "3",
            "--tiers",
            tiers,
        ],
    );
    assert!(stdout.contains("member_root:"), "create output: {stdout}");

    run_ok(
        &dir,
        &[
            "propose",
            "--action",
            "transfer",
            "--recipient",
            RECIPIENT,
            "--amount",
            "500",
            "--tier",
            "1",
        ],
    );

    // One aggregated proof for both members.
    let stdout = run_ok(
        &dir,
        &["approve-all", "--proposal", "0", "--members", "0,1"],
    );
    assert!(
        stdout.contains("aggregated approval recorded for proposal 0 (2 members, one proof)"),
        "approve-all output: {stdout}"
    );
    assert!(
        dir.join("claims").join("claim-0-aggregated.json").exists(),
        "aggregated claim artifact missing"
    );

    run_ok(&dir, &["execute", "--proposal", "0"]);
    let stdout = run_ok(&dir, &["info"]);
    assert!(stdout.contains("Executed"), "info output: {stdout}");
}

#[test]
fn per_member_approval_and_double_vote_rejection() {
    let dir = workdir("per-member");
    let tiers = r#"[{"id":1,"threshold":2,"max_amount":1000}]"#;
    run_ok(
        &dir,
        &[
            "create",
            "--threshold",
            "2",
            "--members",
            "3",
            "--tiers",
            tiers,
        ],
    );
    run_ok(
        &dir,
        &[
            "propose",
            "--action",
            "transfer",
            "--recipient",
            RECIPIENT,
            "--amount",
            "500",
            "--tier",
            "1",
        ],
    );

    let stdout = run_ok(&dir, &["approve", "--member", "0", "--proposal", "0"]);
    assert!(
        stdout.contains("approval recorded"),
        "approve output: {stdout}"
    );
    assert!(dir.join("claims").join("claim-0-0.json").exists());

    // Same member twice → deterministic double-vote rejection.
    let stdout = run_err(&dir, &["approve", "--member", "0", "--proposal", "0"]);
    assert!(
        stdout.contains("duplicate nullifier") || stdout.contains("1005"),
        "expected double-vote rejection, got: {stdout}"
    );

    run_ok(&dir, &["approve", "--member", "1", "--proposal", "0"]);
    run_ok(&dir, &["execute", "--proposal", "0"]);
}

#[test]
fn rotated_member_key_is_dead() {
    let dir = workdir("rotation");
    let tiers = r#"[{"id":1,"threshold":2,"max_amount":1000}]"#;
    run_ok(
        &dir,
        &[
            "create",
            "--threshold",
            "2",
            "--members",
            "3",
            "--tiers",
            tiers,
        ],
    );
    run_ok(
        &dir,
        &[
            "propose",
            "--action",
            "transfer",
            "--recipient",
            RECIPIENT,
            "--amount",
            "500",
            "--tier",
            "1",
        ],
    );
    run_ok(&dir, &["approve", "--member", "0", "--proposal", "0"]);
    run_ok(&dir, &["approve", "--member", "1", "--proposal", "0"]);
    run_ok(&dir, &["execute", "--proposal", "0"]);

    // Rotate to a fresh random 2-member set.
    let stdout = run_ok(&dir, &["new-root", "--members", "2"]);
    let new_root = stdout.trim();
    run_ok(
        &dir,
        &[
            "propose",
            "--action",
            "rotate",
            "--new-member-root",
            new_root,
            "--new-member-count",
            "2",
        ],
    );
    run_ok(&dir, &["approve", "--member", "0", "--proposal", "1"]);
    run_ok(&dir, &["approve", "--member", "2", "--proposal", "1"]);
    run_ok(&dir, &["execute", "--proposal", "1"]);

    let stdout = run_ok(&dir, &["info"]);
    assert!(stdout.contains("constitution v2"), "info output: {stdout}");

    // A member of the OLD set can no longer approve (no valid Merkle path).
    run_ok(
        &dir,
        &[
            "propose",
            "--action",
            "transfer",
            "--recipient",
            RECIPIENT,
            "--amount",
            "100",
            "--tier",
            "1",
        ],
    );
    let stdout = run_err(&dir, &["approve", "--member", "1", "--proposal", "2"]);
    assert!(
        stdout.contains("[3005] member commitment not in member root"),
        "expected invalid-membership rejection, got: {stdout}"
    );

    // Activation is only allowed after the generated root is active. The new
    // members can then approve a fresh proposal under constitution v2.
    run_ok(&dir, &["activate-rotation"]);
    assert!(!dir.join("member-2.json").exists());
    run_ok(
        &dir,
        &[
            "propose",
            "--action",
            "transfer",
            "--recipient",
            RECIPIENT,
            "--amount",
            "100",
            "--tier",
            "1",
        ],
    );
    run_ok(
        &dir,
        &["approve-all", "--proposal", "3", "--members", "0,1"],
    );
    run_ok(&dir, &["execute", "--proposal", "3"]);
}

#[test]
fn rotation_bundle_cannot_activate_early() {
    let dir = workdir("early-rotation");
    run_ok(&dir, &["create", "--threshold", "2", "--members", "3"]);
    run_ok(&dir, &["new-root", "--members", "3"]);
    let error = run_err(&dir, &["activate-rotation"]);
    assert!(
        error.contains("rotation bundle root is not the active constitution root"),
        "expected active-root check, got: {error}"
    );
}

#[test]
fn network_state_is_private_isolated_and_not_overwritten() {
    let dir = workdir("network-state");
    let stdout = run_ok(&dir, &["network", "--target", "local", "prepare"]);
    assert!(stdout.contains("network_state=prepared"));
    assert!(!stdout.contains("secret"));

    let network_dir = dir.join(".quorum-network-local");
    let state_file = network_dir.join("state.json");
    let secrets_file = network_dir.join("secrets.json");
    let claims_dir = network_dir.join("claims");
    assert_eq!(
        std::fs::metadata(&network_dir)
            .expect("network directory")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(&claims_dir)
            .expect("claims directory")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    for path in [state_file, secrets_file] {
        assert_eq!(
            std::fs::metadata(path)
                .expect("private state file")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    let first = run_ok(&dir, &["network", "--target", "local", "deploy"]);
    let second = run_ok(&dir, &["network", "--target", "local", "deploy"]);
    let prepared_hash = |output: &str| {
        output
            .lines()
            .find_map(|line| line.strip_prefix("transaction_hash="))
            .expect("prepared transaction hash")
            .to_owned()
    };
    assert_eq!(prepared_hash(&first), prepared_hash(&second));
    assert!(first.contains("submission=blocked"));

    let error = run_err(&dir, &["network", "--target", "local", "prepare"]);
    assert!(error.contains("state already exists"));
}

#[test]
fn public_testnet_rejects_development_proofs_before_rpc() {
    let dir = workdir("testnet-dev-guard");
    let error = run_err(&dir, &["network", "--target", "testnet", "health"]);
    assert!(error.contains("RISC0_DEV_MODE must be 0 or unset"));
}
