//! Generates a real (`RISC0_DEV_MODE=0`) 2-of-3 threshold proof and prints
//! timing, receipt size, and the pinned image ID. Set `QUORUM_PROOF_OUTPUT` to
//! persist the JSON artifact for the composer's ignored real-receipt test.
//!
//! ```bash
//! RISC0_DEV_MODE=0 cargo run -p quorum-prover --example prove_threshold --release
//! ```

use std::time::Instant;

use quorum_circuit::{ActionData, MemberApprovalWitness, ThresholdWitness};
use quorum_core::merkle::MemberTree;
use quorum_core::nullifier::member_commitment;
use quorum_prover::{ensure_real_proving_mode, prove, threshold_image_id};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ensure_real_proving_mode()?;

    let secrets: Vec<[u8; 32]> = vec![[1; 32], [2; 32], [3; 32]];
    let commitments: Vec<[u8; 32]> = secrets.iter().map(member_commitment).collect();
    let tree = MemberTree::new(&commitments);
    let approval_for = |secret: [u8; 32]| {
        let p = tree
            .proof_for(&member_commitment(&secret))
            .expect("member proof");
        MemberApprovalWitness {
            member_secret: secret,
            viewing_public_key: [0_u8; quorum_core::VIEWING_PUBLIC_KEY_LEN],
            account_identifier: 0,
            leaf_index: p.leaf_index,
            siblings: p.siblings,
        }
    };

    let witness = ThresholdWitness {
        member_root: tree.root(),
        required_threshold: 2,
        approvals: vec![approval_for(secrets[0]), approval_for(secrets[1])],
        action: ActionData::Transfer {
            recipient: [9; 32],
            amount: 500,
            tier_id: 1,
            tier_max_amount: 1_000,
        },
        proposal_id: 7,
        constitution_version: 1,
    };

    println!("member_root:   {}", hex(&tree.root()));
    println!("image_id:      {:?}", threshold_image_id());
    println!("proving 2-of-3 threshold (real mode)...");

    let start = Instant::now();
    let proof = prove(&witness)?;
    let elapsed = start.elapsed();

    println!("proof time:    {elapsed:?}");
    println!("receipt size:  {} bytes", proof.receipt.len());
    println!("approvals:     {}", proof.journal.approval_count);
    let nullifiers: Vec<String> = proof.journal.nullifiers.iter().map(|n| hex(n)).collect();
    println!("nullifiers:    {nullifiers:?}");
    println!("action:        {:?}", proof.journal.action);
    println!("verify ok:     yes (host re-verified receipt)");

    if let Ok(path) = std::env::var("QUORUM_PROOF_OUTPUT") {
        let json = serde_json::to_vec_pretty(&proof)?;
        std::fs::write(&path, json)?;
        println!("proof artifact: {path}");
    }

    // Print the image ID as a rust array so it can be pasted into
    // crates/quorum-image-id/src/lib.rs.
    let words: Vec<String> = threshold_image_id().iter().map(u32::to_string).collect();
    println!("image_id rust: [{}]", words.join(", "));
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .fold(String::new(), |mut s, h| {
            s.push_str(&h);
            s
        })
}
