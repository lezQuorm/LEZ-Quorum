//! Guarded sequencer workflow for local rehearsal and public testnet use.

use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions, Permissions},
    io::Write as _,
    os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
    str::FromStr as _,
    time::Duration,
};

use clap::{Subcommand, ValueEnum};
use common::{transaction::LeeTransaction, HashType};
use lee::{Account, AccountId};
use lee_core::{
    account::{AccountWithMetadata, Nonce},
    encryption::ViewingPublicKey,
    InputAccountIdentity, DUMMY_COMMITMENT_HASH,
};
use quorum_circuit::ActionData;
use quorum_composer::{
    compose_private_approval_with_progress,
    lifecycle::{self, LifecycleAccounts, LifecycleSeeds},
    network::NetworkClient,
    PrivateApprovalPhase, PrivateApprovalRequest,
};
use quorum_core::nullifier::derive_nullifier;
use quorum_gate_core::{
    check_claim, decode_constitution, decode_proposal, OnChainThresholdJournal, ProposalStatus,
    TierPolicy,
};
use quorum_gate_methods::QUORUM_GATE_ID;
use quorum_prover::{ensure_real_proving_mode, verify_receipt, QuorumProof};
use quorum_sdk::{viewing_public_key_for_secret, Member, MemberSet, Multisig};
use rand::{rngs::OsRng, RngCore as _};
use serde::{Deserialize, Serialize};
use token_core::TokenHolding;

const LOCAL_RPC: &str = "http://127.0.0.1:3040";
const TESTNET_RPC: &str = "https://testnet.lez.logos.co";
const RECORDED_DEPLOYMENT: &str =
    "4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43";
const RECORDED_DEPLOYMENT_BLOCK: u64 = 693;
const LEZ_VERSION: &str = "v0.2.2";
const STATE_FILE: &str = "state.json";
const SECRETS_FILE: &str = "secrets.json";
const CLAIMS_DIR: &str = "claims";
const TOKEN_NAME: &str = "QUORUM-DEMO";
const TOTAL_SUPPLY: u128 = 1_000;
const TRANSFER_TIER: u8 = 1;

/// Sequencer target and its state isolation policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum NetworkTarget {
    /// A developer-owned sequencer on the local machine.
    Local,
    /// The public Logos Execution Zone testnet.
    Testnet,
}

impl NetworkTarget {
    fn default_rpc(self) -> &'static str {
        match self {
            Self::Local => LOCAL_RPC,
            Self::Testnet => TESTNET_RPC,
        }
    }

    fn state_dir(self) -> PathBuf {
        match self {
            Self::Local => PathBuf::from(".quorum-network-local"),
            Self::Testnet => PathBuf::from(".quorum-testnet"),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Testnet => "testnet",
        }
    }
}

/// Read, prepare, submit, and reconcile sequencer operations.
#[derive(Subcommand)]
pub enum NetworkCommand {
    /// Check the RPC contract and current network identity.
    Health,
    /// Verify the recorded gate deployment without submitting anything.
    Deployment {
        /// Deployment transaction hash.
        #[arg(long, default_value = RECORDED_DEPLOYMENT)]
        transaction: String,
    },
    /// Generate isolated lifecycle accounts and private member material.
    Prepare {
        /// Required approvals.
        #[arg(long, default_value_t = 2)]
        threshold: u8,
        /// Total members.
        #[arg(long, default_value_t = 3)]
        members: usize,
        /// Treasury funding amount.
        #[arg(long, default_value_t = 750)]
        funding: u128,
        /// Proposed transfer amount.
        #[arg(long, default_value_t = 250)]
        transfer: u64,
    },
    /// Read the current chain and transaction-journal state.
    Status,
    /// Deploy the pinned Quorum gate.
    Deploy {
        /// Permit this public write after reviewing the prepared hash.
        #[arg(long)]
        confirm_public_write: bool,
    },
    /// Initialize the on-chain constitution.
    Initialize {
        /// Permit this public write after reviewing the prepared hash.
        #[arg(long)]
        confirm_public_write: bool,
    },
    /// Create the demo token definition and initial supply.
    CreateToken {
        /// Permit this public write after reviewing the prepared hash.
        #[arg(long)]
        confirm_public_write: bool,
    },
    /// Initialize the public recipient token account.
    InitializeRecipient {
        /// Permit this public write after reviewing the prepared hash.
        #[arg(long)]
        confirm_public_write: bool,
    },
    /// Initialize the program-derived treasury vault.
    InitializeVault {
        /// Permit this public write after reviewing the prepared hash.
        #[arg(long)]
        confirm_public_write: bool,
    },
    /// Fund the treasury vault from the initial token supply.
    Fund {
        /// Permit this public write after reviewing the prepared hash.
        #[arg(long)]
        confirm_public_write: bool,
    },
    /// Open the configured transfer proposal.
    Propose {
        /// Permit this public write after reviewing the prepared hash.
        #[arg(long)]
        confirm_public_write: bool,
    },
    /// Prove and submit one private member approval.
    Approve {
        /// Member index in the protected local set.
        #[arg(long)]
        member: usize,
        /// Proposal id.
        #[arg(long, default_value_t = 0)]
        proposal: u64,
        /// Permit this public write after reviewing the prepared hash.
        #[arg(long)]
        confirm_public_write: bool,
    },
    /// Prove and submit the next unused private member approval.
    ApproveThreshold {
        /// Proposal id.
        #[arg(long, default_value_t = 0)]
        proposal: u64,
        /// Permit this public write after reviewing the prepared hash.
        #[arg(long)]
        confirm_public_write: bool,
    },
    /// Execute an active proposal whose live threshold has been met.
    Execute {
        /// Proposal id.
        #[arg(long, default_value_t = 0)]
        proposal: u64,
        /// Permit this public write after reviewing the prepared hash.
        #[arg(long)]
        confirm_public_write: bool,
    },
    /// Query unknown transactions before any optional exact resubmission.
    Reconcile {
        /// Restrict reconciliation to one journal label.
        #[arg(long)]
        label: Option<String>,
        /// Resubmit the exact saved transaction only after a not-found query.
        #[arg(long, requires = "confirm_public_write")]
        resubmit_unconfirmed: bool,
        /// Permit exact resubmission after reconciliation.
        #[arg(long)]
        confirm_public_write: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PublicAccounts {
    multisig: [u8; 32],
    definition: [u8; 32],
    supply: [u8; 32],
    recipient: [u8; 32],
    proposal: [u8; 32],
    vault: [u8; 32],
}

impl PublicAccounts {
    fn from_lifecycle(accounts: &LifecycleAccounts) -> Self {
        Self {
            multisig: *accounts.multisig.id.value(),
            definition: *accounts.definition.id.value(),
            supply: *accounts.supply.id.value(),
            recipient: *accounts.recipient.id.value(),
            proposal: *accounts.proposal.id.value(),
            vault: *accounts.vault_id.value(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AccountSeeds {
    multisig: [u8; 32],
    definition: [u8; 32],
    supply: [u8; 32],
    recipient: [u8; 32],
    proposal: [u8; 32],
}

impl AccountSeeds {
    fn generate() -> Self {
        Self {
            multisig: random_bytes(),
            definition: random_bytes(),
            supply: random_bytes(),
            recipient: random_bytes(),
            proposal: random_bytes(),
        }
    }

    fn lifecycle(&self) -> LifecycleSeeds {
        LifecycleSeeds {
            multisig: self.multisig,
            definition: self.definition,
            supply: self.supply,
            recipient: self.recipient,
            proposal: self.proposal,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SecretState {
    account_seeds: AccountSeeds,
    members: Vec<Member>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TransactionStatus {
    Prepared,
    Submitted,
    Unknown,
    Confirmed,
    Orphaned,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TransactionRecord {
    hash: String,
    transaction: LeeTransaction,
    status: TransactionStatus,
    block: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NetworkState {
    schema: u8,
    target: NetworkTarget,
    rpc_url: String,
    network_id: Option<String>,
    lez_version: String,
    gate_program_id: [u32; 8],
    accounts: PublicAccounts,
    commitments: Vec<[u8; 32]>,
    multisig: Multisig,
    funding: u128,
    transfer: u64,
    transactions: BTreeMap<String, TransactionRecord>,
}

/// Runs one network command on a dedicated Tokio runtime.
pub fn run(
    target: NetworkTarget,
    rpc_override: Option<String>,
    command: NetworkCommand,
) -> Result<(), String> {
    if target == NetworkTarget::Testnet {
        ensure_real_proving_mode().map_err(|error| format!("public testnet guard: {error}"))?;
    }
    let rpc = rpc_override.unwrap_or_else(|| target.default_rpc().to_owned());
    let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    runtime.block_on(run_async(target, rpc, command))
}

async fn run_async(
    target: NetworkTarget,
    rpc: String,
    command: NetworkCommand,
) -> Result<(), String> {
    match command {
        NetworkCommand::Health => health(&rpc).await,
        NetworkCommand::Deployment { transaction } => deployment(target, &rpc, &transaction).await,
        NetworkCommand::Prepare {
            threshold,
            members,
            funding,
            transfer,
        } => prepare(target, &rpc, threshold, members, funding, transfer),
        NetworkCommand::Status => status(target, &rpc).await,
        NetworkCommand::Deploy {
            confirm_public_write,
        } => {
            write_transaction(
                target,
                &rpc,
                "deploy",
                lifecycle::deploy_gate(),
                confirm_public_write,
            )
            .await?;
            Ok(())
        }
        NetworkCommand::Initialize {
            confirm_public_write,
        } => initialize(target, &rpc, confirm_public_write).await,
        NetworkCommand::CreateToken {
            confirm_public_write,
        } => create_token(target, &rpc, confirm_public_write).await,
        NetworkCommand::InitializeRecipient {
            confirm_public_write,
        } => initialize_recipient(target, &rpc, confirm_public_write).await,
        NetworkCommand::InitializeVault {
            confirm_public_write,
        } => initialize_vault(target, &rpc, confirm_public_write).await,
        NetworkCommand::Fund {
            confirm_public_write,
        } => fund(target, &rpc, confirm_public_write).await,
        NetworkCommand::Propose {
            confirm_public_write,
        } => propose(target, &rpc, confirm_public_write).await,
        NetworkCommand::Approve {
            member,
            proposal,
            confirm_public_write,
        } => approve(target, &rpc, member, proposal, confirm_public_write).await,
        NetworkCommand::ApproveThreshold {
            proposal,
            confirm_public_write,
        } => approve_threshold(target, &rpc, proposal, confirm_public_write).await,
        NetworkCommand::Execute {
            proposal,
            confirm_public_write,
        } => execute(target, &rpc, proposal, confirm_public_write).await,
        NetworkCommand::Reconcile {
            label,
            resubmit_unconfirmed,
            confirm_public_write,
        } => {
            reconcile(
                target,
                &rpc,
                label.as_deref(),
                resubmit_unconfirmed,
                confirm_public_write,
            )
            .await
        }
    }
}

async fn health(rpc: &str) -> Result<(), String> {
    let client = client(rpc)?;
    client
        .check_health()
        .await
        .map_err(|error| error.to_string())?;
    let block = client
        .get_last_block_id()
        .await
        .map_err(|error| error.to_string())?;
    let channel = client
        .get_channel_id()
        .await
        .map_err(|error| error.to_string())?;
    let programs = client
        .get_program_ids()
        .await
        .map_err(|error| error.to_string())?;
    println!("rpc_status=healthy");
    println!("rpc={rpc}");
    println!("lez_target={LEZ_VERSION}");
    println!("network_id={channel}");
    println!("block={block}");
    println!("builtin_programs={}", programs.len());
    Ok(())
}

async fn deployment(target: NetworkTarget, rpc: &str, value: &str) -> Result<(), String> {
    let hash = parse_hash(value)?;
    let client = client(rpc)?;
    let result = find_transaction(&client, hash)
        .await?
        .ok_or_else(|| format!("deployment transaction {hash} was not found"))?;
    let (transaction, block) = result;
    let expected = lifecycle::deploy_gate();
    if transaction != expected {
        return Err("deployed bytecode does not match the pinned Quorum gate".to_owned());
    }
    println!("deployment_status=verified");
    println!("deployment_tx={hash}");
    println!("deployment_block={block}");
    println!("gate_program_id={}", program_id_hex(QUORUM_GATE_ID));

    if let Ok(mut state) = load_state(target, rpc) {
        state.transactions.insert(
            "deploy".to_owned(),
            TransactionRecord {
                hash: hash.to_string(),
                transaction,
                status: TransactionStatus::Confirmed,
                block: Some(block),
            },
        );
        bind_network(&client, &mut state).await?;
        save_state(target, &state)?;
    }
    Ok(())
}

fn prepare(
    target: NetworkTarget,
    rpc: &str,
    threshold: u8,
    members: usize,
    funding: u128,
    transfer: u64,
) -> Result<(), String> {
    if members == 0 || members > 10 {
        return Err("member count must be 1..=10".to_owned());
    }
    if threshold == 0 || usize::from(threshold) > members {
        return Err("threshold must be between 1 and the member count".to_owned());
    }
    if transfer == 0 || u128::from(transfer) > funding {
        return Err("transfer must be positive and no greater than funding".to_owned());
    }

    let directory = target.state_dir();
    let state_path = directory.join(STATE_FILE);
    let secrets_path = directory.join(SECRETS_FILE);
    if state_path.exists() || secrets_path.exists() {
        return Err(format!(
            "{} state already exists; choose a fresh working directory",
            target.label()
        ));
    }
    private_directory(&directory)?;
    private_directory(&directory.join(CLAIMS_DIR))?;

    let account_seeds = AccountSeeds::generate();
    let accounts = LifecycleAccounts::from_seeds(&account_seeds.lifecycle())
        .map_err(|error| error.to_string())?;
    let member_set = MemberSet::generate(members);
    let tiers = vec![TierPolicy {
        id: 1,
        threshold,
        max_amount: funding
            .try_into()
            .map_err(|_| "funding exceeds the supported tier amount")?,
    }];
    let multisig = Multisig::create_with_account_id(
        *accounts.multisig.id.value(),
        threshold,
        &member_set,
        tiers,
    )
    .map_err(|error| error.to_string())?;
    let state = NetworkState {
        schema: 1,
        target,
        rpc_url: rpc.to_owned(),
        network_id: None,
        lez_version: LEZ_VERSION.to_owned(),
        gate_program_id: QUORUM_GATE_ID,
        accounts: PublicAccounts::from_lifecycle(&accounts),
        commitments: member_set.members.iter().map(Member::commitment).collect(),
        multisig,
        funding,
        transfer,
        transactions: BTreeMap::new(),
    };
    let secrets = SecretState {
        account_seeds,
        members: member_set.members,
    };
    write_json(&state_path, &state)?;
    write_json(&secrets_path, &secrets)?;

    println!("network_state=prepared");
    println!("target={}", target.label());
    println!("rpc={rpc}");
    println!("gate_program_id={}", program_id_hex(QUORUM_GATE_ID));
    print_account_ids(&state.accounts);
    println!("threshold={threshold}");
    println!("members={members}");
    println!("funding={funding}");
    println!("transfer={transfer}");
    println!("state_directory={}", directory.display());
    Ok(())
}

async fn status(target: NetworkTarget, rpc: &str) -> Result<(), String> {
    let mut state = load_state(target, rpc)?;
    let client = client(rpc)?;
    client
        .check_health()
        .await
        .map_err(|error| error.to_string())?;
    bind_network(&client, &mut state).await?;
    let block = client
        .get_last_block_id()
        .await
        .map_err(|error| error.to_string())?;
    let orphaned = refresh_transaction_statuses(&client, &mut state).await?;
    println!("rpc_status=healthy");
    println!("target={}", target.label());
    println!(
        "network_id={}",
        state.network_id.as_deref().unwrap_or("unknown")
    );
    println!("block={block}");
    if orphaned.is_empty() {
        println!("journal_status=consistent");
    } else {
        println!("journal_status=orphaned");
        println!("orphaned_transactions={}", orphaned.join(","));
    }
    println!("gate_program_id={}", program_id_hex(QUORUM_GATE_ID));
    print_account_ids(&state.accounts);

    let multisig_id = account_id(state.accounts.multisig);
    let multisig_account = client
        .get_account(multisig_id)
        .await
        .map_err(|error| error.to_string())?;
    if multisig_account == Account::default() {
        println!("constitution_status=not_initialized");
    } else {
        let constitution = decode_constitution(&multisig_account.data)
            .map_err(|error| format!("invalid live constitution: {error}"))?;
        validate_constitution(&state, &constitution)?;
        println!("constitution_status=initialized");
        println!("constitution_version={}", constitution.version);
        println!("proposal_counter={}", constitution.proposal_counter);
        state.multisig.constitution = constitution;
    }

    let proposal_id = account_id(state.accounts.proposal);
    let proposal_account = client
        .get_account(proposal_id)
        .await
        .map_err(|error| error.to_string())?;
    if proposal_account == Account::default() {
        println!("proposal_status=not_initialized");
    } else {
        let proposal = decode_proposal(&proposal_account.data)
            .map_err(|error| format!("invalid live proposal: {error}"))?;
        validate_proposal(&state, &proposal)?;
        println!("proposal_id={}", proposal.id);
        println!("proposal_status={:?}", proposal.status);
        println!("approvals={}", proposal.nullifiers.len());
        println!("required_approvals={}", proposal.threshold);
        upsert_proposal(&mut state.multisig, proposal)?;
    }

    print_holding(&client, "vault", state.accounts.vault).await?;
    print_holding(&client, "recipient", state.accounts.recipient).await?;
    for (label, record) in &state.transactions {
        println!(
            "transaction={label} hash={} status={:?} block={}",
            record.hash,
            record.status,
            record
                .block
                .map_or_else(|| "pending".to_owned(), |value| value.to_string())
        );
    }
    save_state(target, &state)
}

async fn initialize(
    target: NetworkTarget,
    rpc: &str,
    confirm_public_write: bool,
) -> Result<(), String> {
    require_live_confirmation(target, rpc, "deploy").await?;
    let (state, secrets) = load_all(target, rpc)?;
    let accounts = lifecycle_accounts(&state, &secrets)?;
    let transaction = lifecycle::initialize_constitution(
        &accounts,
        Nonce(0),
        state.multisig.constitution.threshold,
        state.multisig.constitution.member_count,
        state.multisig.constitution.member_root,
        state.multisig.constitution.tiers.clone(),
    )
    .map_err(|error| error.to_string())?;
    let confirmed =
        write_transaction(target, rpc, "initialize", transaction, confirm_public_write).await?;
    if confirmed {
        sync_constitution(target, rpc).await?;
    }
    Ok(())
}

async fn create_token(
    target: NetworkTarget,
    rpc: &str,
    confirm_public_write: bool,
) -> Result<(), String> {
    require_live_confirmation(target, rpc, "initialize").await?;
    let (state, secrets) = load_all(target, rpc)?;
    let accounts = lifecycle_accounts(&state, &secrets)?;
    let transaction = lifecycle::create_token(
        &accounts,
        Nonce(0),
        Nonce(0),
        TOKEN_NAME.to_owned(),
        TOTAL_SUPPLY,
    )
    .map_err(|error| error.to_string())?;
    write_transaction(
        target,
        rpc,
        "create-token",
        transaction,
        confirm_public_write,
    )
    .await?;
    Ok(())
}

async fn initialize_recipient(
    target: NetworkTarget,
    rpc: &str,
    confirm_public_write: bool,
) -> Result<(), String> {
    require_live_confirmation(target, rpc, "create-token").await?;
    let (state, secrets) = load_all(target, rpc)?;
    let accounts = lifecycle_accounts(&state, &secrets)?;
    let transaction =
        lifecycle::initialize_recipient(&accounts, Nonce(0)).map_err(|error| error.to_string())?;
    write_transaction(
        target,
        rpc,
        "initialize-recipient",
        transaction,
        confirm_public_write,
    )
    .await?;
    Ok(())
}

async fn initialize_vault(
    target: NetworkTarget,
    rpc: &str,
    confirm_public_write: bool,
) -> Result<(), String> {
    require_live_confirmation(target, rpc, "initialize-recipient").await?;
    let (state, secrets) = load_all(target, rpc)?;
    let accounts = lifecycle_accounts(&state, &secrets)?;
    let client = client(rpc)?;
    let nonce = one_nonce(&client, accounts.multisig.id).await?;
    let transaction =
        lifecycle::initialize_vault(&accounts, nonce).map_err(|error| error.to_string())?;
    write_transaction(
        target,
        rpc,
        "initialize-vault",
        transaction,
        confirm_public_write,
    )
    .await?;
    Ok(())
}

async fn fund(target: NetworkTarget, rpc: &str, confirm_public_write: bool) -> Result<(), String> {
    require_live_confirmation(target, rpc, "initialize-vault").await?;
    let (state, secrets) = load_all(target, rpc)?;
    let accounts = lifecycle_accounts(&state, &secrets)?;
    let client = client(rpc)?;
    let nonce = one_nonce(&client, accounts.supply.id).await?;
    let transaction = lifecycle::fund_vault(&accounts, nonce, state.funding)
        .map_err(|error| error.to_string())?;
    write_transaction(target, rpc, "fund", transaction, confirm_public_write).await?;
    Ok(())
}

async fn propose(
    target: NetworkTarget,
    rpc: &str,
    confirm_public_write: bool,
) -> Result<(), String> {
    require_live_confirmation(target, rpc, "fund").await?;
    let (state, secrets) = load_all(target, rpc)?;
    if !state.multisig.proposals.is_empty() {
        return Err("this prepared lifecycle already has a proposal".to_owned());
    }
    let accounts = lifecycle_accounts(&state, &secrets)?;
    let mut mirror = state.multisig.clone();
    let proposal_id = mirror
        .propose(ActionData::Transfer {
            recipient: state.accounts.recipient,
            amount: state.transfer,
            tier_id: TRANSFER_TIER,
            tier_max_amount: 0,
        })
        .map_err(|error| error.to_string())?;
    if proposal_id != 0 {
        return Err("prepared demo lifecycle must begin with proposal 0".to_owned());
    }
    let action = mirror.proposals[0].action.clone();
    let client = client(rpc)?;
    let nonce = one_nonce(&client, accounts.proposal.id).await?;
    let transaction =
        lifecycle::propose(&accounts, nonce, action).map_err(|error| error.to_string())?;
    let confirmed =
        write_transaction(target, rpc, "propose", transaction, confirm_public_write).await?;
    if confirmed {
        sync_proposal(target, rpc).await?;
    }
    Ok(())
}

async fn approve(
    target: NetworkTarget,
    rpc: &str,
    member_index: usize,
    proposal_id: u64,
    confirm_public_write: bool,
) -> Result<(), String> {
    if proposal_id != 0 {
        return Err("the prepared demo lifecycle supports proposal 0 only".to_owned());
    }
    let label = format!("approve-{proposal_id}-{member_index}");
    require_live_confirmation(target, rpc, "propose").await?;
    let existing = load_state(target, rpc)?;
    if existing.transactions.contains_key(&label) {
        submit_saved(target, rpc, &label, confirm_public_write).await?;
        if confirm_public_write {
            let proposal = sync_proposal(target, rpc).await?;
            println!("approvals={}", proposal.nullifiers.len());
            println!("required_approvals={}", proposal.threshold);
        }
        return Ok(());
    }

    let (state, secrets) = load_all(target, rpc)?;
    let member = secrets
        .members
        .get(member_index)
        .cloned()
        .ok_or_else(|| format!("member index {member_index} out of range"))?;
    let client = client(rpc)?;
    let multisig_id = account_id(state.accounts.multisig);
    let proposal_account_id = account_id(state.accounts.proposal);
    let constitution_account = client
        .get_account(multisig_id)
        .await
        .map_err(|error| error.to_string())?;
    let proposal_account = client
        .get_account(proposal_account_id)
        .await
        .map_err(|error| error.to_string())?;
    let constitution = client
        .get_constitution(multisig_id)
        .await
        .map_err(|error| error.to_string())?;
    let proposal = client
        .get_proposal(proposal_account_id)
        .await
        .map_err(|error| error.to_string())?;
    validate_constitution(&state, &constitution)?;
    validate_proposal(&state, &proposal)?;
    if proposal.status != ProposalStatus::Active {
        return Err("proposal is not active".to_owned());
    }

    let claim_path = target
        .state_dir()
        .join(CLAIMS_DIR)
        .join(format!("claim-{proposal_id}-{member_index}.json"));
    let proof = if let Some(proof) = load_saved_claim(&claim_path, &constitution, &proposal)? {
        print_proof_progress("threshold", "reusing verified threshold receipt");
        proof
    } else {
        let mut mirror = Multisig {
            constitution: constitution.clone(),
            proposals: vec![proposal.clone()],
        };
        print_proof_progress(
            "threshold",
            &format!("proving approval for member {member_index}"),
        );
        let proof = mirror
            .approve(proposal_id, &state.commitments, &member)
            .map_err(|error| error.to_string())?;
        write_json(&claim_path, &proof)?;
        proof
    };

    let composed = compose_network_approval(
        &state,
        &[&member],
        constitution_account,
        proposal_account,
        proposal_id,
        &proof,
    )?;
    let confirmed = write_transaction(
        target,
        rpc,
        &label,
        LeeTransaction::PrivacyPreserving(composed.transaction),
        confirm_public_write,
    )
    .await?;
    if confirmed {
        let proposal = sync_proposal(target, rpc).await?;
        println!("approvals={}", proposal.nullifiers.len());
        println!("required_approvals={}", proposal.threshold);
    }
    Ok(())
}

async fn approve_threshold(
    target: NetworkTarget,
    rpc: &str,
    proposal_id: u64,
    confirm_public_write: bool,
) -> Result<(), String> {
    if proposal_id != 0 {
        return Err("the prepared demo lifecycle supports proposal 0 only".to_owned());
    }
    require_live_confirmation(target, rpc, "propose").await?;
    let existing = load_state(target, rpc)?;
    let legacy_label = format!("approve-{proposal_id}-threshold");
    if existing.transactions.contains_key(&legacy_label) {
        submit_saved(target, rpc, &legacy_label, confirm_public_write).await?;
        if confirm_public_write {
            let proposal = sync_proposal(target, rpc).await?;
            println!("approvals={}", proposal.nullifiers.len());
            println!("required_approvals={}", proposal.threshold);
        }
        return Ok(());
    }

    let (state, secrets) = load_all(target, rpc)?;
    let client = client(rpc)?;
    let multisig_id = account_id(state.accounts.multisig);
    let proposal_account_id = account_id(state.accounts.proposal);
    let constitution = client
        .get_constitution(multisig_id)
        .await
        .map_err(|error| error.to_string())?;
    let proposal = client
        .get_proposal(proposal_account_id)
        .await
        .map_err(|error| error.to_string())?;
    validate_constitution(&state, &constitution)?;
    validate_proposal(&state, &proposal)?;
    if proposal.status != ProposalStatus::Active {
        return Err("proposal is not active".to_owned());
    }

    let member_index = pending_member_indexes(&proposal, &constitution, &secrets)?
        .into_iter()
        .next()
        .ok_or_else(|| "no unused local member is available".to_owned())?;
    approve(target, rpc, member_index, proposal_id, confirm_public_write).await
}

fn load_saved_claim(
    path: &Path,
    constitution: &quorum_gate_core::ConstitutionState,
    proposal: &quorum_gate_core::ProposalState,
) -> Result<Option<QuorumProof>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let proof: QuorumProof = read_json(path)?;
    let journal = verify_receipt(&proof).map_err(|error| error.to_string())?;
    let onchain_journal = OnChainThresholdJournal::from(&journal);
    if check_claim(constitution, proposal, &onchain_journal).is_err() {
        return Ok(None);
    }
    Ok(Some(proof))
}

fn pending_member_indexes(
    proposal: &quorum_gate_core::ProposalState,
    constitution: &quorum_gate_core::ConstitutionState,
    secrets: &SecretState,
) -> Result<Vec<usize>, String> {
    let required = usize::from(proposal.threshold);
    let remaining = required.saturating_sub(proposal.nullifiers.len());
    if remaining == 0 {
        return Err("proposal threshold is already met".to_owned());
    }
    let indexes = secrets
        .members
        .iter()
        .enumerate()
        .filter_map(|(index, member)| {
            let nullifier = derive_nullifier(&member.secret, proposal.id, constitution.version);
            (!proposal.nullifiers.contains(&nullifier)).then_some(index)
        })
        .take(remaining)
        .collect::<Vec<_>>();
    if indexes.len() != remaining {
        return Err(format!(
            "only {} unused local members are available for {remaining} remaining approvals",
            indexes.len()
        ));
    }
    Ok(indexes)
}

fn compose_network_approval(
    state: &NetworkState,
    members: &[&Member],
    constitution_account: Account,
    proposal_account: Account,
    proposal_id: u64,
    proof: &quorum_prover::QuorumProof,
) -> Result<quorum_composer::ComposedApproval, String> {
    let multisig_id = account_id(state.accounts.multisig);
    let proposal_account_id = account_id(state.accounts.proposal);
    let mut pre_states = vec![
        AccountWithMetadata::new(constitution_account, false, multisig_id),
        AccountWithMetadata::new(proposal_account, false, proposal_account_id),
    ];
    let mut account_identities = vec![InputAccountIdentity::Public, InputAccountIdentity::Public];
    for member in members {
        let credential_id = member.account_id();
        let vpk =
            ViewingPublicKey::from_bytes(viewing_public_key_for_secret(&member.secret).to_vec())
                .map_err(|error| error.to_string())?;
        pre_states.push(AccountWithMetadata::new(
            Account::default(),
            true,
            account_id(credential_id),
        ));
        account_identities.push(InputAccountIdentity::PrivateAuthorizedInit {
            vpk,
            random_seed: random_bytes(),
            nsk: member.secret,
            identifier: member.account_identifier,
            commitment_root: DUMMY_COMMITMENT_HASH,
        });
    }
    compose_private_approval_with_progress(
        PrivateApprovalRequest {
            programs: lifecycle::gate_program()
                .map_err(|error| error.to_string())?
                .into(),
            pre_states,
            account_identities,
            dummy_inputs: Vec::new(),
            public_account_ids: vec![multisig_id, proposal_account_id],
            public_nonces: Vec::new(),
            public_signers: Vec::new(),
            proposal_id,
        },
        proof,
        |phase| match phase {
            PrivateApprovalPhase::GateProgram => {
                print_proof_progress("gate", "proving Quorum gate execution");
            }
            PrivateApprovalPhase::PrivacyCircuit => {
                print_proof_progress("privacy", "proving final LEZ private transaction");
            }
        },
    )
    .map_err(|error| error.to_string())
}

fn print_proof_progress(phase: &str, detail: &str) {
    println!("proof_phase={phase}");
    println!("proof_detail={detail}");
    let _ = std::io::stdout().flush();
}

async fn execute(
    target: NetworkTarget,
    rpc: &str,
    proposal_id: u64,
    confirm_public_write: bool,
) -> Result<(), String> {
    if proposal_id != 0 {
        return Err("the prepared demo lifecycle supports proposal 0 only".to_owned());
    }
    require_live_confirmation(target, rpc, "propose").await?;
    let (state, secrets) = load_all(target, rpc)?;
    let client = client(rpc)?;
    let proposal = client
        .get_proposal(account_id(state.accounts.proposal))
        .await
        .map_err(|error| error.to_string())?;
    validate_proposal(&state, &proposal)?;
    if proposal.status != ProposalStatus::Active {
        return Err("proposal is not active".to_owned());
    }
    if !proposal.threshold_met() {
        return Err(format!(
            "proposal threshold not met: {}/{} approvals",
            proposal.nullifiers.len(),
            proposal.threshold
        ));
    }
    let accounts = lifecycle_accounts(&state, &secrets)?;
    let transaction =
        lifecycle::execute(&accounts, proposal_id).map_err(|error| error.to_string())?;
    let confirmed =
        write_transaction(target, rpc, "execute", transaction, confirm_public_write).await?;
    if confirmed {
        verify_final_state(target, rpc).await?;
    }
    Ok(())
}

async fn write_transaction(
    target: NetworkTarget,
    rpc: &str,
    label: &str,
    transaction: LeeTransaction,
    confirm_public_write: bool,
) -> Result<bool, String> {
    let mut state = load_state(target, rpc)?;
    let hash = if let Some(record) = state.transactions.get(label) {
        record.hash.clone()
    } else {
        let hash = transaction.hash().to_string();
        state.transactions.insert(
            label.to_owned(),
            TransactionRecord {
                hash: hash.clone(),
                transaction,
                status: TransactionStatus::Prepared,
                block: None,
            },
        );
        save_state(target, &state)?;
        hash
    };
    println!("transaction_label={label}");
    println!("transaction_hash={hash}");
    if !confirm_public_write {
        println!("submission=blocked");
        println!("next=review the hash, then repeat with --confirm-public-write");
        return Ok(false);
    }
    submit_saved(target, rpc, label, true).await?;
    let state = load_state(target, rpc)?;
    Ok(state
        .transactions
        .get(label)
        .is_some_and(|record| record.status == TransactionStatus::Confirmed))
}

async fn submit_saved(
    target: NetworkTarget,
    rpc: &str,
    label: &str,
    confirm_public_write: bool,
) -> Result<(), String> {
    if !confirm_public_write {
        let state = load_state(target, rpc)?;
        let record = state
            .transactions
            .get(label)
            .ok_or_else(|| format!("transaction {label} is not prepared"))?;
        println!("transaction_label={label}");
        println!("transaction_hash={}", record.hash);
        println!("submission=blocked");
        println!("next=review the hash, then repeat with --confirm-public-write");
        return Ok(());
    }
    let mut state = load_state(target, rpc)?;
    let client = client(rpc)?;
    bind_network(&client, &mut state).await?;
    let record = state
        .transactions
        .get(label)
        .cloned()
        .ok_or_else(|| format!("transaction {label} is not prepared"))?;
    let hash = parse_hash(&record.hash)?;
    if let Some(block) = locate_saved_transaction(&client, &record).await? {
        mark_confirmed(&mut state, label, block)?;
        save_state(target, &state)?;
        print_confirmation(label, hash, block);
        return Ok(());
    }
    let missing_status = mark_not_found(&mut state, label)?;
    if missing_status == TransactionStatus::Orphaned {
        save_state(target, &state)?;
        return Err(format!(
            "transaction {label} was confirmed locally but is absent from the current chain; start a fresh {} session",
            target.label()
        ));
    }
    if record.status != TransactionStatus::Prepared {
        save_state(target, &state)?;
        return Err(format!(
            "transaction {label} is {:?} and was not found; use reconcile before exact resubmission",
            record.status
        ));
    }

    match client.submit_transaction(record.transaction).await {
        Ok(returned_hash) if returned_hash == hash => {
            state
                .transactions
                .get_mut(label)
                .ok_or_else(|| format!("transaction {label} disappeared"))?
                .status = TransactionStatus::Submitted;
            save_state(target, &state)?;
        }
        Ok(returned_hash) => {
            state
                .transactions
                .get_mut(label)
                .ok_or_else(|| format!("transaction {label} disappeared"))?
                .status = TransactionStatus::Unknown;
            save_state(target, &state)?;
            return Err(format!(
                "sequencer returned {returned_hash}, expected prepared hash {hash}"
            ));
        }
        Err(error) => {
            state
                .transactions
                .get_mut(label)
                .ok_or_else(|| format!("transaction {label} disappeared"))?
                .status = TransactionStatus::Unknown;
            save_state(target, &state)?;
            return Err(format!(
                "submission outcome is unknown for {hash}: {error}; reconcile before retrying"
            ));
        }
    }

    match client.wait_for_transaction(hash).await {
        Ok(block) => {
            mark_confirmed(&mut state, label, block)?;
            save_state(target, &state)?;
            print_confirmation(label, hash, block);
            Ok(())
        }
        Err(error) => {
            state
                .transactions
                .get_mut(label)
                .ok_or_else(|| format!("transaction {label} disappeared"))?
                .status = TransactionStatus::Unknown;
            save_state(target, &state)?;
            Err(format!("{error}; reconcile before retrying"))
        }
    }
}

async fn reconcile(
    target: NetworkTarget,
    rpc: &str,
    selected: Option<&str>,
    resubmit: bool,
    confirmed: bool,
) -> Result<(), String> {
    if resubmit && !confirmed {
        return Err("--resubmit-unconfirmed requires --confirm-public-write".to_owned());
    }
    let mut state = load_state(target, rpc)?;
    let client = client(rpc)?;
    bind_network(&client, &mut state).await?;
    let labels = state
        .transactions
        .keys()
        .filter(|label| selected.is_none_or(|value| value == label.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if labels.is_empty() {
        return Err("no matching journal transactions".to_owned());
    }

    for label in labels {
        let record = state
            .transactions
            .get(&label)
            .cloned()
            .ok_or_else(|| format!("transaction {label} disappeared"))?;
        let hash = parse_hash(&record.hash)?;
        if let Some(block) = locate_saved_transaction(&client, &record).await? {
            mark_confirmed(&mut state, &label, block)?;
            print_confirmation(&label, hash, block);
            continue;
        }
        let missing_status = mark_not_found(&mut state, &label)?;
        println!("transaction_label={label}");
        println!("transaction_hash={hash}");
        println!(
            "transaction_status={}",
            if missing_status == TransactionStatus::Orphaned {
                "orphaned"
            } else {
                "not_found"
            }
        );
        if resubmit {
            if missing_status == TransactionStatus::Orphaned {
                save_state(target, &state)?;
                return Err(format!(
                    "refusing to resubmit orphaned transaction {label}; start a fresh {} session",
                    target.label()
                ));
            }
            let returned = client
                .submit_transaction(record.transaction)
                .await
                .map_err(|error| format!("exact resubmission failed: {error}"))?;
            if returned != hash {
                return Err(format!(
                    "sequencer returned {returned}, expected journal hash {hash}"
                ));
            }
            state
                .transactions
                .get_mut(&label)
                .ok_or_else(|| format!("transaction {label} disappeared"))?
                .status = TransactionStatus::Submitted;
            save_state(target, &state)?;
            let block = client
                .wait_for_transaction(hash)
                .await
                .map_err(|error| error.to_string())?;
            mark_confirmed(&mut state, &label, block)?;
            print_confirmation(&label, hash, block);
        }
    }
    save_state(target, &state)
}

async fn refresh_transaction_statuses(
    client: &NetworkClient,
    state: &mut NetworkState,
) -> Result<Vec<String>, String> {
    let labels = state.transactions.keys().cloned().collect::<Vec<_>>();
    let mut orphaned = Vec::new();
    for label in labels {
        let record = state
            .transactions
            .get(&label)
            .cloned()
            .ok_or_else(|| format!("transaction {label} disappeared"))?;
        if let Some(block) = locate_saved_transaction(client, &record).await? {
            mark_confirmed(state, &label, block)?;
        } else if mark_not_found(state, &label)? == TransactionStatus::Orphaned {
            orphaned.push(label);
        }
    }
    Ok(orphaned)
}

async fn require_live_confirmation(
    target: NetworkTarget,
    rpc: &str,
    label: &str,
) -> Result<(), String> {
    let mut state = load_state(target, rpc)?;
    let client = client(rpc)?;
    bind_network(&client, &mut state).await?;
    let record = state
        .transactions
        .get(label)
        .cloned()
        .ok_or_else(|| format!("{label} must be confirmed first"))?;
    if let Some(block) = locate_saved_transaction(&client, &record).await? {
        mark_confirmed(&mut state, label, block)?;
        save_state(target, &state)?;
        return Ok(());
    }

    let missing_status = mark_not_found(&mut state, label)?;
    save_state(target, &state)?;
    if missing_status == TransactionStatus::Orphaned {
        Err(format!(
            "{label} confirmation is orphaned: transaction {} is absent from the current chain; start a fresh {} session",
            record.hash,
            target.label()
        ))
    } else {
        Err(format!(
            "{label} must be confirmed on the current chain first"
        ))
    }
}

async fn locate_saved_transaction(
    client: &NetworkClient,
    record: &TransactionRecord,
) -> Result<Option<u64>, String> {
    let hash = parse_hash(&record.hash)?;
    let Some((transaction, block)) = find_transaction(client, hash).await? else {
        return Ok(None);
    };
    if transaction != record.transaction {
        return Err(format!(
            "transaction {} does not match the saved journal bytes",
            record.hash
        ));
    }
    Ok(Some(block))
}

fn mark_not_found(state: &mut NetworkState, label: &str) -> Result<TransactionStatus, String> {
    let record = state
        .transactions
        .get_mut(label)
        .ok_or_else(|| format!("transaction {label} disappeared"))?;
    if record.status == TransactionStatus::Confirmed {
        record.status = TransactionStatus::Orphaned;
    }
    Ok(record.status)
}

async fn find_transaction(
    client: &NetworkClient,
    hash: HashType,
) -> Result<Option<(LeeTransaction, u64)>, String> {
    if let Some(transaction) = client
        .get_transaction(hash)
        .await
        .map_err(|error| error.to_string())?
    {
        return Ok(Some(transaction));
    }
    if hash.to_string() != RECORDED_DEPLOYMENT {
        return Ok(None);
    }
    client
        .get_transaction_in_block(hash, RECORDED_DEPLOYMENT_BLOCK)
        .await
        .map_err(|error| error.to_string())
        .map(|transaction| transaction.map(|value| (value, RECORDED_DEPLOYMENT_BLOCK)))
}

async fn sync_constitution(target: NetworkTarget, rpc: &str) -> Result<(), String> {
    let mut state = load_state(target, rpc)?;
    let client = client(rpc)?;
    let constitution = client
        .get_constitution(account_id(state.accounts.multisig))
        .await
        .map_err(|error| error.to_string())?;
    validate_constitution(&state, &constitution)?;
    state.multisig.constitution = constitution;
    save_state(target, &state)
}

async fn sync_proposal(
    target: NetworkTarget,
    rpc: &str,
) -> Result<quorum_gate_core::ProposalState, String> {
    let mut state = load_state(target, rpc)?;
    let client = client(rpc)?;
    let proposal = client
        .get_proposal(account_id(state.accounts.proposal))
        .await
        .map_err(|error| error.to_string())?;
    validate_proposal(&state, &proposal)?;
    upsert_proposal(&mut state.multisig, proposal.clone())?;
    save_state(target, &state)?;
    Ok(proposal)
}

async fn verify_final_state(target: NetworkTarget, rpc: &str) -> Result<(), String> {
    sync_proposal(target, rpc).await?;
    let state = load_state(target, rpc)?;
    let client = client(rpc)?;
    let proposal = client
        .get_proposal(account_id(state.accounts.proposal))
        .await
        .map_err(|error| error.to_string())?;
    if proposal.status != ProposalStatus::Executed {
        return Err("proposal was confirmed but is not Executed".to_owned());
    }
    let vault = client
        .get_token_holding(account_id(state.accounts.vault))
        .await
        .map_err(|error| error.to_string())?;
    let recipient = client
        .get_token_holding(account_id(state.accounts.recipient))
        .await
        .map_err(|error| error.to_string())?;
    let expected_vault = state
        .funding
        .checked_sub(u128::from(state.transfer))
        .ok_or("configured transfer exceeds funding")?;
    validate_balance(&vault, state.accounts.definition, expected_vault, "vault")?;
    validate_balance(
        &recipient,
        state.accounts.definition,
        u128::from(state.transfer),
        "recipient",
    )?;
    println!("vault_balance={expected_vault}");
    println!("recipient_balance={}", state.transfer);
    println!("proposal_status=Executed");
    println!("RESULT=PASS");
    Ok(())
}

fn validate_balance(
    holding: &TokenHolding,
    definition: [u8; 32],
    expected: u128,
    label: &str,
) -> Result<(), String> {
    let expected_holding = TokenHolding::Fungible {
        definition_id: account_id(definition),
        balance: expected,
    };
    if holding != &expected_holding {
        return Err(format!("{label} token definition or balance mismatch"));
    }
    Ok(())
}

fn validate_constitution(
    state: &NetworkState,
    constitution: &quorum_gate_core::ConstitutionState,
) -> Result<(), String> {
    if constitution.multisig_id != state.accounts.multisig
        || constitution.version != state.multisig.constitution.version
        || constitution.member_root != state.multisig.constitution.member_root
        || constitution.member_count != state.multisig.constitution.member_count
        || constitution.threshold != state.multisig.constitution.threshold
        || constitution.tiers != state.multisig.constitution.tiers
    {
        return Err("live constitution does not match the prepared state".to_owned());
    }
    Ok(())
}

fn validate_proposal(
    state: &NetworkState,
    proposal: &quorum_gate_core::ProposalState,
) -> Result<(), String> {
    if proposal.multisig_id != state.accounts.multisig || proposal.id != 0 {
        return Err("live proposal belongs to another constitution or id".to_owned());
    }
    let tier = state
        .multisig
        .constitution
        .tiers
        .iter()
        .find(|tier| tier.id == TRANSFER_TIER)
        .ok_or("prepared transfer tier is missing")?;
    let expected_action = ActionData::Transfer {
        recipient: state.accounts.recipient,
        amount: state.transfer,
        tier_id: TRANSFER_TIER,
        tier_max_amount: tier.max_amount,
    };
    if proposal.constitution_version != state.multisig.constitution.version
        || proposal.threshold != tier.threshold
        || proposal.action != expected_action
    {
        return Err("live proposal policy or transfer does not match prepared state".to_owned());
    }
    Ok(())
}

fn upsert_proposal(
    multisig: &mut Multisig,
    proposal: quorum_gate_core::ProposalState,
) -> Result<(), String> {
    let index = usize::try_from(proposal.id).map_err(|_| "proposal id overflow")?;
    if index > multisig.proposals.len() {
        return Err("proposal sequence has a gap".to_owned());
    }
    if index == multisig.proposals.len() {
        multisig.proposals.push(proposal);
    } else {
        multisig.proposals[index] = proposal;
    }
    Ok(())
}

async fn print_holding(client: &NetworkClient, label: &str, id: [u8; 32]) -> Result<(), String> {
    let account = client
        .get_account(account_id(id))
        .await
        .map_err(|error| error.to_string())?;
    if account == Account::default() {
        println!("{label}_balance=not_initialized");
        return Ok(());
    }
    match TokenHolding::try_from(&account.data)
        .map_err(|error| format!("invalid live {label} token account: {error}"))?
    {
        TokenHolding::Fungible { balance, .. } => println!("{label}_balance={balance}"),
        _ => println!("{label}_balance=non_fungible"),
    }
    Ok(())
}

async fn bind_network(client: &NetworkClient, state: &mut NetworkState) -> Result<(), String> {
    let current = client
        .get_channel_id()
        .await
        .map_err(|error| error.to_string())?;
    match state.network_id.as_deref() {
        Some(expected) if expected != current => Err(format!(
            "network mismatch: expected {expected}, received {current}"
        )),
        Some(_) => Ok(()),
        None => {
            state.network_id = Some(current);
            Ok(())
        }
    }
}

async fn one_nonce(client: &NetworkClient, id: AccountId) -> Result<Nonce, String> {
    let mut nonces = client
        .get_account_nonces(vec![id])
        .await
        .map_err(|error| error.to_string())?;
    if nonces.len() != 1 {
        return Err("sequencer returned an unexpected nonce count".to_owned());
    }
    Ok(nonces.remove(0))
}

fn lifecycle_accounts(
    state: &NetworkState,
    secrets: &SecretState,
) -> Result<LifecycleAccounts, String> {
    let accounts = LifecycleAccounts::from_seeds(&secrets.account_seeds.lifecycle())
        .map_err(|error| error.to_string())?;
    let derived = PublicAccounts::from_lifecycle(&accounts);
    if derived.multisig != state.accounts.multisig
        || derived.definition != state.accounts.definition
        || derived.supply != state.accounts.supply
        || derived.recipient != state.accounts.recipient
        || derived.proposal != state.accounts.proposal
        || derived.vault != state.accounts.vault
    {
        return Err("private account seeds do not match public state".to_owned());
    }
    Ok(accounts)
}

fn load_all(target: NetworkTarget, rpc: &str) -> Result<(NetworkState, SecretState), String> {
    let state = load_state(target, rpc)?;
    let secrets: SecretState = read_json(&target.state_dir().join(SECRETS_FILE))?;
    lifecycle_accounts(&state, &secrets)?;
    Ok((state, secrets))
}

fn load_state(target: NetworkTarget, rpc: &str) -> Result<NetworkState, String> {
    let state: NetworkState = read_json(&target.state_dir().join(STATE_FILE))?;
    if state.schema != 1 || state.target != target || state.rpc_url != rpc {
        return Err("network state target, RPC, or schema mismatch".to_owned());
    }
    validate_pins(&state)?;
    Ok(state)
}

fn validate_pins(state: &NetworkState) -> Result<(), String> {
    if state.lez_version != LEZ_VERSION || state.gate_program_id != QUORUM_GATE_ID {
        return Err("network state is pinned to another LEZ or gate version".to_owned());
    }
    Ok(())
}

fn save_state(target: NetworkTarget, state: &NetworkState) -> Result<(), String> {
    write_json(&target.state_dir().join(STATE_FILE), state)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    write_private(path, &bytes)
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        private_directory(parent)?;
    }
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("invalid private file path: {}", path.display()))?;
    let temporary = path.with_file_name(format!(".{filename}.{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(&temporary, path).map_err(|error| error.to_string())?;
    fs::set_permissions(path, Permissions::from_mode(0o600)).map_err(|error| error.to_string())
}

fn private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| error.to_string())?;
    fs::set_permissions(path, Permissions::from_mode(0o700)).map_err(|error| error.to_string())
}

fn client(rpc: &str) -> Result<NetworkClient, String> {
    NetworkClient::connect(rpc)
        .map(|client| {
            client.with_confirmation_policy(Duration::from_secs(2), Duration::from_secs(90))
        })
        .map_err(|error| error.to_string())
}

fn account_id(value: [u8; 32]) -> AccountId {
    AccountId::new(value)
}

fn parse_hash(value: &str) -> Result<HashType, String> {
    HashType::from_str(value).map_err(|error| format!("invalid transaction hash: {error}"))
}

fn mark_confirmed(state: &mut NetworkState, label: &str, block: u64) -> Result<(), String> {
    let record = state
        .transactions
        .get_mut(label)
        .ok_or_else(|| format!("transaction {label} disappeared"))?;
    record.status = TransactionStatus::Confirmed;
    record.block = Some(block);
    Ok(())
}

fn print_confirmation(label: &str, hash: HashType, block: u64) {
    println!("transaction_label={label}");
    println!("transaction_hash={hash}");
    println!("transaction_status=confirmed");
    println!("confirmation_block={block}");
}

fn print_account_ids(accounts: &PublicAccounts) {
    println!("multisig=Public/{}", account_id(accounts.multisig));
    println!("definition=Public/{}", account_id(accounts.definition));
    println!("supply=Public/{}", account_id(accounts.supply));
    println!("recipient=Public/{}", account_id(accounts.recipient));
    println!("vault=Public/{}", account_id(accounts.vault));
    println!("proposal=Public/{}", account_id(accounts.proposal));
}

fn program_id_hex(words: [u32; 8]) -> String {
    let bytes = words
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    hex::encode(bytes)
}

fn random_bytes() -> [u8; 32] {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared_state() -> (NetworkState, SecretState) {
        let account_seeds = AccountSeeds {
            multisig: [11; 32],
            definition: [12; 32],
            supply: [13; 32],
            recipient: [14; 32],
            proposal: [15; 32],
        };
        let accounts = LifecycleAccounts::from_seeds(&account_seeds.lifecycle()).unwrap();
        let members = MemberSet::from_secrets(&[[21; 32], [22; 32], [23; 32]]);
        let multisig = Multisig::create_with_account_id(
            *accounts.multisig.id.value(),
            2,
            &members,
            vec![TierPolicy {
                id: TRANSFER_TIER,
                threshold: 2,
                max_amount: 750,
            }],
        )
        .unwrap();
        (
            NetworkState {
                schema: 1,
                target: NetworkTarget::Testnet,
                rpc_url: TESTNET_RPC.to_owned(),
                network_id: None,
                lez_version: LEZ_VERSION.to_owned(),
                gate_program_id: QUORUM_GATE_ID,
                accounts: PublicAccounts::from_lifecycle(&accounts),
                commitments: members.members.iter().map(Member::commitment).collect(),
                multisig,
                funding: 750,
                transfer: 250,
                transactions: BTreeMap::new(),
            },
            SecretState {
                account_seeds,
                members: members.members,
            },
        )
    }

    #[test]
    fn target_state_is_isolated() {
        assert_ne!(
            NetworkTarget::Local.state_dir(),
            NetworkTarget::Testnet.state_dir()
        );
        assert_ne!(
            NetworkTarget::Local.default_rpc(),
            NetworkTarget::Testnet.default_rpc()
        );
    }

    #[test]
    fn balance_validation_checks_definition_and_amount() {
        let definition = [7_u8; 32];
        let holding = TokenHolding::Fungible {
            definition_id: account_id(definition),
            balance: 500,
        };
        assert!(validate_balance(&holding, definition, 500, "vault").is_ok());
        assert!(validate_balance(&holding, definition, 499, "vault").is_err());
        assert!(validate_balance(&holding, [8_u8; 32], 500, "vault").is_err());
    }

    #[test]
    fn gate_program_id_uses_canonical_byte_order() {
        assert_eq!(
            program_id_hex(QUORUM_GATE_ID),
            "f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec"
        );
    }

    #[test]
    fn version_program_and_vault_mismatches_are_rejected() {
        let (state, secrets) = prepared_state();
        assert!(validate_pins(&state).is_ok());
        assert!(lifecycle_accounts(&state, &secrets).is_ok());

        let mut wrong_version = state.clone();
        wrong_version.lez_version = "v0.2.1".to_owned();
        assert!(validate_pins(&wrong_version).is_err());

        let mut wrong_program = state.clone();
        wrong_program.gate_program_id[0] ^= 1;
        assert!(validate_pins(&wrong_program).is_err());

        let mut wrong_vault = state;
        wrong_vault.accounts.vault[0] ^= 1;
        assert!(lifecycle_accounts(&wrong_vault, &secrets).is_err());
    }

    #[test]
    fn proposal_recipient_and_constitution_mismatches_are_rejected() {
        let (state, _) = prepared_state();
        assert!(validate_constitution(&state, &state.multisig.constitution).is_ok());

        let mut wrong_constitution = state.multisig.constitution.clone();
        wrong_constitution.member_root[0] ^= 1;
        assert!(validate_constitution(&state, &wrong_constitution).is_err());

        let mut wrong_tiers = state.multisig.constitution.clone();
        wrong_tiers.tiers[0].max_amount += 1;
        assert!(validate_constitution(&state, &wrong_tiers).is_err());

        let mut mirror = state.multisig.clone();
        mirror
            .propose(ActionData::Transfer {
                recipient: state.accounts.recipient,
                amount: state.transfer,
                tier_id: TRANSFER_TIER,
                tier_max_amount: 0,
            })
            .unwrap();
        let proposal = &mirror.proposals[0];
        assert!(validate_proposal(&state, proposal).is_ok());

        let mut wrong_recipient = proposal.clone();
        if let ActionData::Transfer { recipient, .. } = &mut wrong_recipient.action {
            recipient[0] ^= 1;
        }
        assert!(validate_proposal(&state, &wrong_recipient).is_err());

        let mut wrong_policy = proposal.clone();
        wrong_policy.threshold += 1;
        assert!(validate_proposal(&state, &wrong_policy).is_err());
    }

    #[test]
    fn threshold_approval_selects_only_unused_members() {
        let (state, secrets) = prepared_state();
        let mut mirror = state.multisig.clone();
        mirror
            .propose(ActionData::Transfer {
                recipient: state.accounts.recipient,
                amount: state.transfer,
                tier_id: TRANSFER_TIER,
                tier_max_amount: 0,
            })
            .unwrap();
        let constitution = mirror.constitution.clone();
        let mut proposal = mirror.proposals[0].clone();

        assert_eq!(
            pending_member_indexes(&proposal, &constitution, &secrets).unwrap(),
            vec![0, 1]
        );

        proposal.nullifiers.push(derive_nullifier(
            &secrets.members[0].secret,
            proposal.id,
            constitution.version,
        ));
        assert_eq!(
            pending_member_indexes(&proposal, &constitution, &secrets).unwrap(),
            vec![1]
        );

        proposal.nullifiers.push(derive_nullifier(
            &secrets.members[1].secret,
            proposal.id,
            constitution.version,
        ));
        assert!(pending_member_indexes(&proposal, &constitution, &secrets).is_err());
    }

    #[test]
    fn missing_confirmation_is_marked_orphaned() {
        let (mut state, _) = prepared_state();
        state.transactions.insert(
            "deploy".to_owned(),
            TransactionRecord {
                hash: lifecycle::deploy_gate().hash().to_string(),
                transaction: lifecycle::deploy_gate(),
                status: TransactionStatus::Confirmed,
                block: Some(42),
            },
        );

        assert_eq!(
            mark_not_found(&mut state, "deploy").unwrap(),
            TransactionStatus::Orphaned
        );
        let record = state.transactions.get("deploy").unwrap();
        assert_eq!(record.status, TransactionStatus::Orphaned);
        assert_eq!(record.block, Some(42));
    }

    #[test]
    fn missing_prepared_transaction_remains_resubmittable() {
        let (mut state, _) = prepared_state();
        state.transactions.insert(
            "deploy".to_owned(),
            TransactionRecord {
                hash: lifecycle::deploy_gate().hash().to_string(),
                transaction: lifecycle::deploy_gate(),
                status: TransactionStatus::Prepared,
                block: None,
            },
        );

        assert_eq!(
            mark_not_found(&mut state, "deploy").unwrap(),
            TransactionStatus::Prepared
        );
    }
}
