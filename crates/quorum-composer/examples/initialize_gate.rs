//! Prepares a deterministic 1-of-1 testnet constitution.
//! The embedded test key is public and must not control funds.

use std::time::Duration;

use anyhow::{ensure, Context as _, Result};
use common::transaction::LeeTransaction;
use lee::{
    program::Program,
    public_transaction::{Message as PublicMessage, WitnessSet as PublicWitnessSet},
    AccountId, PrivateKey, PublicKey, PublicTransaction,
};
use quorum_composer::network::NetworkClient;
use quorum_gate_core::QuorumInstruction;
use quorum_gate_methods::QUORUM_GATE_ID;

#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let rpc_url = args
        .first()
        .cloned()
        .unwrap_or_else(|| "https://testnet.lez.logos.co".to_owned());
    let seed = args
        .get(1)
        .map(|value| value.parse::<u8>().context("seed must be an integer"))
        .transpose()?
        .unwrap_or(201);
    let submit = args.iter().any(|argument| argument == "--submit");
    ensure!(seed > 0 && seed <= 250, "seed must be from 1 to 250");

    let key = PrivateKey::try_new([seed; 32]).context("invalid deterministic private key")?;
    let account_id = AccountId::from(&PublicKey::new_from_private_key(&key));
    let instruction = Program::serialize_instruction(QuorumInstruction::Initialize {
        threshold: 1,
        member_count: 1,
        member_root: [seed; 32],
        tiers: vec![],
    })
    .context("initialize instruction")?;
    let message = PublicMessage::new_preserialized(
        QUORUM_GATE_ID,
        vec![account_id],
        vec![0_u128.into()],
        instruction,
    );
    let witness_set = PublicWitnessSet::for_message(&message, &[&key]);
    let transaction = LeeTransaction::Public(PublicTransaction::new(message, witness_set));
    let expected_hash = transaction.hash();

    println!("rpc={rpc_url}");
    println!("gate_program_id={QUORUM_GATE_ID:?}");
    println!("multisig=Public/{account_id}");
    println!("initialize_tx={expected_hash}");
    if !submit {
        println!("submission=disabled (pass --submit to broadcast)");
        println!("RESULT=PASS");
        return Ok(());
    }

    let client = NetworkClient::connect(&rpc_url)?
        .with_confirmation_policy(Duration::from_secs(2), Duration::from_secs(180));
    let hash = client
        .submit_transaction_and_confirm(transaction)
        .await
        .context("gate initialization")?;
    ensure!(
        hash == expected_hash,
        "sequencer returned an unexpected hash"
    );
    let account = client
        .get_account(account_id)
        .await
        .context("initialized gate account")?;

    println!("account_owner={:?}", account.program_owner);
    println!("RESULT=PASS");
    Ok(())
}
