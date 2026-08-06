use std::{borrow::Cow, time::Duration};

use anyhow::{ensure, Context as _, Result};
use common::transaction::LeeTransaction;
use lee::{
    program::Program,
    program_deployment_transaction::Message as DeploymentMessage,
    public_transaction::{Message as PublicMessage, WitnessSet as PublicWitnessSet},
    Account, AccountId, PrivateKey, ProgramDeploymentTransaction, PublicKey, PublicTransaction,
};
use lee_core::{
    account::{AccountWithMetadata, Nonce},
    encryption::ViewingPublicKey,
    program::{InstructionData, PdaSeed, ProgramId},
    InputAccountIdentity,
};
use quorum_circuit::{
    evaluate, ActionData, MemberApprovalWitness, ThresholdJournal, ThresholdWitness,
};
use quorum_composer::{compose_private_approval, network::NetworkClient, PrivateApprovalRequest};
use quorum_core::{merkle::MemberTree, nullifier::member_commitment_for_credential};
use quorum_gate_core::{
    decode_constitution, decode_proposal, vault_pda_seed, ProposalStatus, QuorumInstruction,
    TierPolicy,
};
use quorum_gate_methods::{QUORUM_GATE_ELF, QUORUM_GATE_ID};
use quorum_prover::QuorumProof;
use quorum_threshold_methods::THRESHOLD_ELF;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use sha2::{Digest as _, Sha256};
use token_core::{Instruction as TokenInstruction, TokenHolding};

const TOTAL_SUPPLY: u128 = 1_000;
const TREASURY_FUNDING: u128 = 750;
const TRANSFER_AMOUNT: u64 = 250;

struct PublicAccount {
    key: PrivateKey,
    id: AccountId,
}

impl PublicAccount {
    fn from_byte(byte: u8) -> Result<Self> {
        let key = PrivateKey::try_new([byte; 32]).context("invalid deterministic private key")?;
        let id = AccountId::from(&PublicKey::new_from_private_key(&key));
        Ok(Self { key, id })
    }
}

fn public_transaction(
    program_id: ProgramId,
    account_ids: Vec<AccountId>,
    nonces: Vec<Nonce>,
    signers: &[&PrivateKey],
    instruction_data: InstructionData,
) -> LeeTransaction {
    let message =
        PublicMessage::new_preserialized(program_id, account_ids, nonces, instruction_data);
    let witnesses = PublicWitnessSet::for_message(&message, signers);
    LeeTransaction::Public(PublicTransaction::new(message, witnesses))
}

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

struct LifecycleAccounts {
    multisig: PublicAccount,
    definition: PublicAccount,
    supply: PublicAccount,
    recipient: PublicAccount,
    proposal: PublicAccount,
    vault_id: AccountId,
}

impl LifecycleAccounts {
    fn from_seed(key_seed: u8) -> Result<Self> {
        let multisig = PublicAccount::from_byte(key_seed)?;
        let vault_id = AccountId::for_public_pda(
            &QUORUM_GATE_ID,
            &PdaSeed::new(vault_pda_seed(multisig.id.value())),
        );
        Ok(Self {
            multisig,
            definition: PublicAccount::from_byte(key_seed + 1)?,
            supply: PublicAccount::from_byte(key_seed + 2)?,
            recipient: PublicAccount::from_byte(key_seed + 3)?,
            proposal: PublicAccount::from_byte(key_seed + 4)?,
            vault_id,
        })
    }

    fn print_ids(&self) {
        println!("gate_program_id={QUORUM_GATE_ID:?}");
        println!("multisig=Public/{}", self.multisig.id);
        println!("vault=Public/{}", self.vault_id);
        println!("recipient=Public/{}", self.recipient.id);
    }
}

fn gate_program() -> Result<Program> {
    let program = Program::new(Cow::Borrowed(QUORUM_GATE_ELF)).context("gate program")?;
    ensure!(program.id() == QUORUM_GATE_ID, "embedded gate ID mismatch");
    Ok(program)
}

async fn initialize_constitution(
    client: &NetworkClient,
    accounts: &LifecycleAccounts,
    witness: &ThresholdWitness,
) -> Result<()> {
    submit(
        client,
        "deploy",
        LeeTransaction::ProgramDeployment(ProgramDeploymentTransaction::new(
            DeploymentMessage::new(QUORUM_GATE_ELF.to_vec()),
        )),
    )
    .await?;

    let instruction = Program::serialize_instruction(QuorumInstruction::Initialize {
        threshold: 2,
        member_count: 3,
        member_root: witness.member_root,
        tiers: vec![TierPolicy {
            id: 1,
            threshold: 2,
            max_amount: 1_000,
        }],
    })?;
    submit(
        client,
        "initialize",
        public_transaction(
            QUORUM_GATE_ID,
            vec![accounts.multisig.id],
            vec![0_u128.into()],
            &[&accounts.multisig.key],
            instruction,
        ),
    )
    .await
}

async fn initialize_token_accounts(
    client: &NetworkClient,
    accounts: &LifecycleAccounts,
) -> Result<()> {
    let token_program_id = programs::token().id();
    let create = Program::serialize_instruction(TokenInstruction::NewFungibleDefinition {
        name: "QUORUM-DEMO".to_owned(),
        total_supply: TOTAL_SUPPLY,
    })?;
    submit(
        client,
        "create_token",
        public_transaction(
            token_program_id,
            vec![accounts.definition.id, accounts.supply.id],
            vec![0_u128.into(), 0_u128.into()],
            &[&accounts.definition.key, &accounts.supply.key],
            create,
        ),
    )
    .await?;

    let recipient = Program::serialize_instruction(TokenInstruction::InitializeAccount)?;
    submit(
        client,
        "initialize_recipient",
        public_transaction(
            token_program_id,
            vec![accounts.definition.id, accounts.recipient.id],
            vec![0_u128.into()],
            &[&accounts.recipient.key],
            recipient,
        ),
    )
    .await?;

    let vault = Program::serialize_instruction(QuorumInstruction::InitializeVault)?;
    submit(
        client,
        "initialize_vault",
        public_transaction(
            QUORUM_GATE_ID,
            vec![
                accounts.multisig.id,
                accounts.definition.id,
                accounts.vault_id,
            ],
            vec![1_u128.into()],
            &[&accounts.multisig.key],
            vault,
        ),
    )
    .await?;

    let funding = Program::serialize_instruction(TokenInstruction::Transfer {
        amount_to_transfer: TREASURY_FUNDING,
    })?;
    submit(
        client,
        "fund_vault",
        public_transaction(
            token_program_id,
            vec![accounts.supply.id, accounts.vault_id],
            vec![1_u128.into()],
            &[&accounts.supply.key],
            funding,
        ),
    )
    .await
}

async fn propose_transfer(
    client: &NetworkClient,
    accounts: &LifecycleAccounts,
    witness: &ThresholdWitness,
) -> Result<()> {
    let instruction = Program::serialize_instruction(QuorumInstruction::Propose {
        action: witness.action.clone(),
    })?;
    submit(
        client,
        "propose",
        public_transaction(
            QUORUM_GATE_ID,
            vec![accounts.multisig.id, accounts.proposal.id],
            vec![0_u128.into()],
            &[&accounts.proposal.key],
            instruction,
        ),
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
            programs: gate_program()?.into(),
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
    let instruction =
        Program::serialize_instruction(QuorumInstruction::Execute { proposal_id: 0 })?;
    submit(
        client,
        "execute",
        public_transaction(
            QUORUM_GATE_ID,
            vec![
                accounts.multisig.id,
                accounts.proposal.id,
                accounts.vault_id,
                accounts.recipient.id,
            ],
            Vec::new(),
            &[],
            instruction,
        ),
    )
    .await?;

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
    let accounts = LifecycleAccounts::from_seed(key_seed)?;
    accounts.print_ids();
    gate_program()?;
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
