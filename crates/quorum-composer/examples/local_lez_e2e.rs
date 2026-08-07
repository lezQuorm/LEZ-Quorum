use std::time::Duration;

use anyhow::{ensure, Context as _, Result};
use common::transaction::LeeTransaction;
use lee::{Account, AccountId};
use lee_core::{
    account::{AccountWithMetadata, Nonce},
    encryption::ViewingPublicKey,
    InputAccountIdentity,
};
use quorum_circuit::{
    evaluate, ActionData, MemberApprovalWitness, ThresholdJournal, ThresholdWitness,
};
use quorum_composer::{
    compose_private_approval,
    lifecycle::{self, LifecycleAccounts, LifecycleSeeds},
    network::NetworkClient,
    PrivateApprovalRequest,
};
use quorum_core::{merkle::MemberTree, nullifier::member_commitment_for_credential};
use quorum_gate_core::{decode_constitution, decode_proposal, ProposalStatus, TierPolicy};
use quorum_gate_methods::QUORUM_GATE_ID;
use quorum_prover::QuorumProof;
use quorum_threshold_methods::THRESHOLD_ELF;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use sha2::{Digest as _, Sha256};
use token_core::TokenHolding;

const TOTAL_SUPPLY: u128 = 1_000;
const TREASURY_FUNDING: u128 = 750;
const TRANSFER_AMOUNT: u64 = 250;

fn credential_secret(key_seed: u8, member_index: u8) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"quorum/local-e2e/credential/v1");
    hasher.update([key_seed, member_index]);
    hasher.finalize().into()
}

fn viewing_public_key(key_byte: u8) -> ViewingPublicKey {
    ViewingPublicKey::from_seed(&[key_byte; 32], &[key_byte + 1; 32])
}

fn viewing_public_key_bytes(key_byte: u8) -> [u8; quorum_core::VIEWING_PUBLIC_KEY_LEN] {
    viewing_public_key(key_byte)
        .to_bytes()
        .try_into()
        .expect("official viewing public key length")
}

fn approval_witness(
    recipient: AccountId,
    key_seed: u8,
) -> (ThresholdWitness, Vec<[u8; 32]>, [[u8; 32]; 2]) {
    let secrets = [
        credential_secret(key_seed, 0),
        credential_secret(key_seed, 1),
        credential_secret(key_seed, 2),
    ];
    let viewing_public_keys = [
        viewing_public_key_bytes(31),
        viewing_public_key_bytes(41),
        viewing_public_key_bytes(51),
    ];
    let commitments = secrets
        .iter()
        .zip(&viewing_public_keys)
        .map(|(secret, viewing_public_key)| {
            member_commitment_for_credential(secret, viewing_public_key, 0)
        })
        .collect::<Vec<_>>();
    let tree = MemberTree::new(&commitments);
    let approval_for =
        |secret: [u8; 32], viewing_public_key: [u8; quorum_core::VIEWING_PUBLIC_KEY_LEN]| {
            let commitment = member_commitment_for_credential(&secret, &viewing_public_key, 0);
            let path = tree.proof_for(&commitment).expect("member path");
            MemberApprovalWitness {
                member_secret: secret,
                viewing_public_key,
                account_identifier: 0,
                leaf_index: path.leaf_index,
                siblings: path.siblings,
            }
        };
    let witness = ThresholdWitness {
        member_root: tree.root(),
        required_threshold: 2,
        approvals: vec![
            approval_for(secrets[0], viewing_public_keys[0]),
            approval_for(secrets[1], viewing_public_keys[1]),
        ],
        action: ActionData::Transfer {
            recipient: *recipient.value(),
            amount: TRANSFER_AMOUNT,
            tier_id: 1,
            tier_max_amount: 1_000,
        },
        proposal_id: 0,
        constitution_version: 1,
    };
    let credential_ids = secrets[..2]
        .iter()
        .zip(&viewing_public_keys)
        .map(|(secret, viewing_public_key)| {
            lez_compat::private_account_id(secret, viewing_public_key, 0)
        })
        .collect();
    (witness, credential_ids, [secrets[0], secrets[1]])
}

fn prove_threshold(witness: &ThresholdWitness) -> Result<QuorumProof> {
    let expected = evaluate(witness).context("threshold witness validation")?;
    let env = ExecutorEnv::builder()
        .write(witness)
        .context("threshold input")?
        .build()
        .context("threshold environment")?;
    let info = default_prover()
        .prove_with_opts(env, THRESHOLD_ELF, &ProverOpts::succinct())
        .context("threshold proof")?;
    let journal = info
        .receipt
        .journal
        .decode::<ThresholdJournal>()
        .context("threshold journal")?;
    ensure!(journal == expected, "threshold prover journal mismatch");
    Ok(QuorumProof {
        journal,
        receipt: bincode::serialize(&info.receipt).context("threshold receipt serialization")?,
    })
}

fn private_init_identity(nsk: [u8; 32], key_byte: u8) -> InputAccountIdentity {
    let vpk = viewing_public_key(key_byte);
    InputAccountIdentity::PrivateAuthorizedInit {
        vpk,
        random_seed: [key_byte + 2; 32],
        nsk,
        identifier: 0,
        commitment_root: lee_core::DUMMY_COMMITMENT_HASH,
    }
}

async fn submit(client: &NetworkClient, label: &str, transaction: LeeTransaction) -> Result<()> {
    let hash = client
        .submit_transaction_and_confirm(transaction)
        .await
        .with_context(|| format!("{label} transaction"))?;
    println!("{label}_tx={hash}");
    Ok(())
}

fn lifecycle_accounts(key_seed: u8) -> Result<LifecycleAccounts> {
    LifecycleAccounts::from_seeds(&LifecycleSeeds {
        multisig: [key_seed; 32],
        definition: [key_seed + 1; 32],
        supply: [key_seed + 2; 32],
        recipient: [key_seed + 3; 32],
        proposal: [key_seed + 4; 32],
    })
    .context("deterministic lifecycle accounts")
}

fn print_ids(accounts: &LifecycleAccounts) {
    println!("gate_program_id={QUORUM_GATE_ID:?}");
    println!("multisig=Public/{}", accounts.multisig.id);
    println!("vault=Public/{}", accounts.vault_id);
    println!("recipient=Public/{}", accounts.recipient.id);
}

async fn initialize_constitution(
    client: &NetworkClient,
    accounts: &LifecycleAccounts,
    witness: &ThresholdWitness,
) -> Result<()> {
    submit(client, "deploy", lifecycle::deploy_gate()).await?;

    submit(
        client,
        "initialize",
        lifecycle::initialize_constitution(
            accounts,
            Nonce(0),
            2,
            3,
            witness.member_root,
            vec![TierPolicy {
                id: 1,
                threshold: 2,
                max_amount: 1_000,
            }],
        )?,
    )
    .await
}

async fn initialize_token_accounts(
    client: &NetworkClient,
    accounts: &LifecycleAccounts,
) -> Result<()> {
    submit(
        client,
        "create_token",
        lifecycle::create_token(
            accounts,
            Nonce(0),
            Nonce(0),
            "QUORUM-DEMO".to_owned(),
            TOTAL_SUPPLY,
        )?,
    )
    .await?;

    submit(
        client,
        "initialize_recipient",
        lifecycle::initialize_recipient(accounts, Nonce(0))?,
    )
    .await?;

    submit(
        client,
        "initialize_vault",
        lifecycle::initialize_vault(accounts, Nonce(1))?,
    )
    .await?;

    submit(
        client,
        "fund_vault",
        lifecycle::fund_vault(accounts, Nonce(1), TREASURY_FUNDING)?,
    )
    .await
}

async fn propose_transfer(
    client: &NetworkClient,
    accounts: &LifecycleAccounts,
    witness: &ThresholdWitness,
) -> Result<()> {
    submit(
        client,
        "propose",
        lifecycle::propose(accounts, Nonce(0), witness.action.clone())?,
    )
    .await
}

async fn submit_private_approval(
    client: &NetworkClient,
    accounts: &LifecycleAccounts,
    witness: &ThresholdWitness,
    credential_ids: &[[u8; 32]],
    credential_secrets: &[[u8; 32]; 2],
) -> Result<()> {
    let proof = prove_threshold(witness)?;
    let multisig_state = client.get_account(accounts.multisig.id).await?;
    let proposal_state = client.get_account(accounts.proposal.id).await?;
    let mut pre_states = vec![
        AccountWithMetadata::new(multisig_state, false, accounts.multisig.id),
        AccountWithMetadata::new(proposal_state, false, accounts.proposal.id),
    ];
    pre_states.extend(
        credential_ids
            .iter()
            .map(|id| AccountWithMetadata::new(Account::default(), true, AccountId::new(*id))),
    );
    let composed = compose_private_approval(
        PrivateApprovalRequest {
            programs: lifecycle::gate_program()?.into(),
            pre_states,
            account_identities: vec![
                InputAccountIdentity::Public,
                InputAccountIdentity::Public,
                private_init_identity(credential_secrets[0], 31),
                private_init_identity(credential_secrets[1], 41),
            ],
            dummy_inputs: vec![],
            public_account_ids: vec![accounts.multisig.id, accounts.proposal.id],
            public_nonces: Vec::new(),
            public_signers: Vec::new(),
            proposal_id: 0,
        },
        &proof,
    )?;
    let hash = client.submit_and_confirm(composed.transaction).await?;
    println!("approve_tx={hash}");

    let account = client.get_account(accounts.proposal.id).await?;
    let proposal = decode_proposal(&account.data).context("approved proposal state")?;
    ensure!(
        proposal.threshold_met(),
        "approval threshold was not recorded"
    );
    Ok(())
}

async fn execute_and_verify(client: &NetworkClient, accounts: &LifecycleAccounts) -> Result<()> {
    submit(client, "execute", lifecycle::execute(accounts, 0)?).await?;

    let multisig = client.get_account(accounts.multisig.id).await?;
    let proposal = client.get_account(accounts.proposal.id).await?;
    let vault = client.get_account(accounts.vault_id).await?;
    let recipient = client.get_account(accounts.recipient.id).await?;
    let constitution = decode_constitution(&multisig.data).context("final constitution state")?;
    let proposal_state = decode_proposal(&proposal.data).context("final proposal state")?;
    let vault_holding = TokenHolding::try_from(&vault.data).context("vault token holding")?;
    let recipient_holding =
        TokenHolding::try_from(&recipient.data).context("recipient token holding")?;

    ensure!(
        constitution.proposal_counter == 1,
        "proposal counter mismatch"
    );
    ensure!(
        proposal_state.status == ProposalStatus::Executed,
        "proposal not executed"
    );
    ensure!(
        vault_holding
            == TokenHolding::Fungible {
                definition_id: accounts.definition.id,
                balance: TREASURY_FUNDING - u128::from(TRANSFER_AMOUNT),
            },
        "vault balance mismatch"
    );
    ensure!(
        recipient_holding
            == TokenHolding::Fungible {
                definition_id: accounts.definition.id,
                balance: u128::from(TRANSFER_AMOUNT),
            },
        "recipient balance mismatch"
    );

    println!(
        "vault_balance={}",
        TREASURY_FUNDING - u128::from(TRANSFER_AMOUNT)
    );
    println!("recipient_balance={TRANSFER_AMOUNT}");
    println!("proposal_status=Executed");
    println!("RESULT=PASS");
    Ok(())
}

fn arguments() -> Result<(String, u8)> {
    let mut args = std::env::args().skip(1);
    let rpc_url = args
        .next()
        .unwrap_or_else(|| "http://127.0.0.1:3040".to_owned());
    let key_seed = args
        .next()
        .map(|value| value.parse::<u8>())
        .transpose()
        .context("identity seed must be an integer from 1 to 250")?
        .unwrap_or(51);
    ensure!(
        key_seed > 0 && key_seed <= 250,
        "account seed must be from 1 to 250"
    );
    Ok((rpc_url, key_seed))
}

#[tokio::main]
async fn main() -> Result<()> {
    let (rpc_url, key_seed) = arguments()?;
    let proof_mode = if std::env::var_os("RISC0_DEV_MODE").as_deref() == Some("1".as_ref()) {
        "development"
    } else {
        "real"
    };
    println!("rpc={rpc_url}");
    println!("proof_mode={proof_mode}");

    let client = NetworkClient::connect(&rpc_url)?
        .with_confirmation_policy(Duration::from_secs(1), Duration::from_secs(90));
    let accounts = lifecycle_accounts(key_seed)?;
    print_ids(&accounts);
    lifecycle::gate_program()?;
    let (witness, credential_ids, credential_secrets) =
        approval_witness(accounts.recipient.id, key_seed);

    initialize_constitution(&client, &accounts, &witness).await?;
    initialize_token_accounts(&client, &accounts).await?;
    propose_transfer(&client, &accounts, &witness).await?;
    submit_private_approval(
        &client,
        &accounts,
        &witness,
        &credential_ids,
        &credential_secrets,
    )
    .await?;
    execute_and_verify(&client, &accounts).await
}
