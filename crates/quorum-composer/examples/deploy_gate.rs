use std::time::Duration;

use anyhow::{Context as _, Result};
use common::transaction::LeeTransaction;
use lee::{
    program_deployment_transaction::Message as DeploymentMessage, ProgramDeploymentTransaction,
};
use quorum_composer::network::NetworkClient;
use quorum_gate_methods::{QUORUM_GATE_ELF, QUORUM_GATE_ID};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let rpc_url = args
        .next()
        .unwrap_or_else(|| "https://testnet.lez.logos.co".to_owned());
    let dry_run = args.next().as_deref() == Some("--dry-run");
    let transaction = LeeTransaction::ProgramDeployment(ProgramDeploymentTransaction::new(
        DeploymentMessage::new(QUORUM_GATE_ELF.to_vec()),
    ));
    let expected_hash = transaction.hash();

    println!("rpc={rpc_url}");
    println!("gate_program_id={QUORUM_GATE_ID:?}");
    println!("deployment_tx={expected_hash}");
    if dry_run {
        println!("RESULT=PASS");
        return Ok(());
    }

    let client = NetworkClient::connect(&rpc_url)?
        .with_confirmation_policy(Duration::from_secs(2), Duration::from_secs(180));
    let hash = client
        .submit_transaction_and_confirm(transaction)
        .await
        .context("gate deployment")?;
    anyhow::ensure!(
        hash == expected_hash,
        "sequencer returned an unexpected hash"
    );
    println!("RESULT=PASS");
    Ok(())
}
