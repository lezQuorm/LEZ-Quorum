//! Private LEZ transaction composition for Quorum approvals.
//!
//! An approve instruction cannot carry a Risc0 receipt as instruction bytes.
//! The threshold receipt must be verified locally and attached as an
//! assumption while proving the gate program. That unconditional gate receipt
//! is then attached to the LEZ privacy-preserving execution circuit.

use std::collections::VecDeque;

use lee::{
    privacy_preserving_transaction::{
        circuit::{ProgramWithDependencies, Proof as LeeProof},
        Message as PrivateMessage, PrivacyPreservingTransaction, WitnessSet,
    },
    program::Program,
    PrivateKey, PRIVACY_PRESERVING_CIRCUIT_ELF,
};
use lee_core::{
    account::{AccountId, AccountWithMetadata, Nonce},
    program::{ChainedCall, InstructionData, ProgramId, ProgramOutput},
    DummyInput, InputAccountIdentity, PrivacyPreservingCircuitInput,
    PrivacyPreservingCircuitOutput,
};
use quorum_gate_core::{
    validate_credentials, OnChainThresholdJournal, QuorumInstruction, ThresholdClaim,
};
use quorum_prover::{decode_receipt, verify_receipt, QuorumProof};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, Receipt};
use thiserror::Error;

pub mod lifecycle;

const MAX_CHAINED_CALLS: usize = 10;

/// Errors produced before or during private transaction composition.
#[derive(Debug, Error)]
pub enum ComposerError {
    /// The threshold proof artifact is malformed or invalid.
    #[error("threshold proof: {0}")]
    ThresholdProof(#[from] quorum_prover::ProverError),
    /// The requested proposal differs from the receipt journal.
    #[error("proof targets proposal {actual}, expected {expected}")]
    ProposalMismatch {
        /// Proposal requested by the caller.
        expected: u64,
        /// Proposal committed by the receipt.
        actual: u64,
    },
    /// Credential accounts do not match the proof journal.
    #[error("credential accounts do not match the proof")]
    CredentialMismatch,
    /// The account list is not valid for the approve instruction.
    #[error("approve requires multisig, proposal, and at least one credential account")]
    MissingAccounts,
    /// Account identities and program pre-states are positionally inconsistent.
    #[error("account identity count does not match program pre-state count")]
    IdentityCountMismatch,
    /// LEZ instruction serialization failed.
    #[error("instruction serialization: {0}")]
    InstructionSerialization(String),
    /// Risc0 input preparation failed.
    #[error("executor input: {0}")]
    ExecutorInput(String),
    /// Program proving failed.
    #[error("program proving: {0}")]
    ProgramProving(String),
    /// Program output could not be decoded.
    #[error("program output: {0}")]
    ProgramOutput(String),
    /// A chained call references a program not declared by the composer.
    #[error("undeclared chained-call program")]
    UndeclaredDependency,
    /// The program exceeded LEZ's chained-call depth.
    #[error("maximum chained-call depth exceeded")]
    MaxChainedCalls,
    /// The LEZ privacy circuit failed.
    #[error("privacy circuit: {0}")]
    PrivacyCircuit(String),
    /// Private transaction message construction failed.
    #[error("private transaction message: {0}")]
    PrivateMessage(String),
    /// Lifecycle account key construction failed.
    #[error("lifecycle account key: {0}")]
    AccountKey(String),
    /// Lifecycle instruction construction failed.
    #[error("lifecycle instruction: {0}")]
    LifecycleInstruction(String),
    /// Embedded gate program construction failed.
    #[error("gate program: {0}")]
    GateProgram(String),
}

/// A locally verified approval ready for gate execution.
pub struct PreparedApproval {
    /// Gate instruction containing only the public receipt journal.
    pub instruction: QuorumInstruction,
    /// Decoded and image-verified threshold receipt.
    pub receipt: Receipt,
    /// Credential account ids bound to the journal.
    pub credential_account_ids: Vec<[u8; 32]>,
}

/// Inputs supplied by the wallet after it has fetched current LEZ state.
pub struct PrivateApprovalRequest {
    /// Gate program plus every program reachable through chained calls.
    pub programs: ProgramWithDependencies,
    /// Gate accounts in IDL order: multisig, proposal, then credentials.
    pub pre_states: Vec<AccountWithMetadata>,
    /// LEZ private/public identity witness for every pre-state.
    pub account_identities: Vec<InputAccountIdentity>,
    /// Randomized inputs used to pad private-account count; empty disables padding.
    pub dummy_inputs: Vec<DummyInput>,
    /// Public accounts included in the final private transaction message.
    pub public_account_ids: Vec<AccountId>,
    /// Current nonces for the public signer accounts.
    pub public_nonces: Vec<Nonce>,
    /// Keys for the public signer accounts, if any.
    pub public_signers: Vec<PrivateKey>,
    /// Proposal id expected by the caller.
    pub proposal_id: u64,
}

/// Result of composing a private approval transaction.
pub struct ComposedApproval {
    /// Transaction ready for sequencer submission.
    pub transaction: PrivacyPreservingTransaction,
    /// Verified journal used by the approve instruction.
    pub journal: OnChainThresholdJournal,
    /// Private credential ids the wallet must reconcile by scanning encrypted
    /// outputs and tracking the transaction's new commitments.
    pub credential_account_ids: Vec<[u8; 32]>,
}

/// Verifies a proof artifact and binds it to private credential account ids.
///
/// # Errors
/// Returns `ComposerError` for malformed receipts, image or journal mismatches,
/// the wrong proposal, or credential substitution.
pub fn prepare_approval(
    proof: &QuorumProof,
    proposal_id: u64,
    credential_account_ids: Vec<[u8; 32]>,
) -> Result<PreparedApproval, ComposerError> {
    let journal = verify_receipt(proof)?;
    if journal.proposal_id != proposal_id {
        return Err(ComposerError::ProposalMismatch {
            expected: proposal_id,
            actual: journal.proposal_id,
        });
    }
    let onchain_journal = OnChainThresholdJournal::from(&journal);
    validate_credentials(&onchain_journal, &credential_account_ids)
        .map_err(|_| ComposerError::CredentialMismatch)?;
    let receipt = decode_receipt(&proof.receipt)?;
    Ok(PreparedApproval {
        instruction: QuorumInstruction::Approve {
            proposal_id,
            claim: ThresholdClaim {
                journal: onchain_journal,
            },
        },
        receipt,
        credential_account_ids,
    })
}

/// Builds a private LEZ approval transaction with recursively resolved receipts.
///
/// # Errors
/// Returns `ComposerError` if preflight validation, gate proving, chained
/// execution, or the outer LEZ privacy proof fails.
pub fn compose_private_approval(
    request: PrivateApprovalRequest,
    proof: &QuorumProof,
) -> Result<ComposedApproval, ComposerError> {
    if request.pre_states.len() < 3 {
        return Err(ComposerError::MissingAccounts);
    }
    if request.pre_states.len() != request.account_identities.len() {
        return Err(ComposerError::IdentityCountMismatch);
    }
    let credential_account_ids = request.pre_states[2..]
        .iter()
        .map(|account| *account.account_id.value())
        .collect::<Vec<_>>();
    let prepared = prepare_approval(proof, request.proposal_id, credential_account_ids)?;
    let journal = match &prepared.instruction {
        QuorumInstruction::Approve { claim, .. } => claim.journal.clone(),
        _ => unreachable!("prepare_approval always creates Approve"),
    };
    let instruction_data = Program::serialize_instruction(&prepared.instruction)
        .map_err(|error| ComposerError::InstructionSerialization(error.to_string()))?;
    let (output, lee_proof) = execute_and_prove_private(
        request.pre_states,
        instruction_data,
        request.account_identities,
        request.dummy_inputs,
        request.programs,
        prepared.receipt,
    )?;
    let message = PrivateMessage::from_circuit_output(request.public_nonces, output);
    if message.public_account_ids() != request.public_account_ids {
        return Err(ComposerError::PrivateMessage(
            "circuit public accounts do not match the requested accounts".to_owned(),
        ));
    }
    let signer_refs = request.public_signers.iter().collect::<Vec<_>>();
    let witness_set = WitnessSet::for_message(&message, lee_proof, &signer_refs);
    Ok(ComposedApproval {
        transaction: PrivacyPreservingTransaction::new(message, witness_set),
        journal,
        credential_account_ids: prepared.credential_account_ids,
    })
}

fn execute_and_prove_private(
    pre_states: Vec<AccountWithMetadata>,
    instruction_data: InstructionData,
    account_identities: Vec<InputAccountIdentity>,
    dummy_inputs: Vec<DummyInput>,
    programs: ProgramWithDependencies,
    threshold_receipt: Receipt,
) -> Result<(PrivacyPreservingCircuitOutput, LeeProof), ComposerError> {
    let ProgramWithDependencies {
        program: initial_program,
        dependencies,
    } = programs;
    let initial_program_id = initial_program.id();
    let mut privacy_env = ExecutorEnv::builder();
    let initial_call = ChainedCall {
        program_id: initial_program_id,
        instruction_data,
        pre_states,
        pda_seeds: vec![],
    };
    let mut calls =
        VecDeque::from([(initial_call, initial_program, None, Some(threshold_receipt))]);
    let mut program_outputs = Vec::new();
    let mut call_count = 0_usize;

    while let Some((call, program, caller_program_id, assumption)) = calls.pop_front() {
        if call_count >= MAX_CHAINED_CALLS {
            return Err(ComposerError::MaxChainedCalls);
        }
        let receipt = prove_program(
            &program,
            caller_program_id,
            &call.pre_states,
            &call.instruction_data,
            assumption,
        )?;
        let output = receipt
            .journal
            .decode::<ProgramOutput>()
            .map_err(|error| ComposerError::ProgramOutput(error.to_string()))?;
        privacy_env.add_assumption(receipt);
        for chained in output.chained_calls.iter().rev() {
            let dependency = dependencies
                .get(&chained.program_id)
                .cloned()
                .ok_or(ComposerError::UndeclaredDependency)?;
            calls.push_front((chained.clone(), dependency, Some(call.program_id), None));
        }
        program_outputs.push(output);
        call_count = call_count
            .checked_add(1)
            .ok_or(ComposerError::MaxChainedCalls)?;
    }

    privacy_env
        .write(&PrivacyPreservingCircuitInput {
            program_outputs,
            account_identities,
            program_id: initial_program_id,
            dummy_inputs,
        })
        .map_err(|error| ComposerError::ExecutorInput(error.to_string()))?;
    let privacy_env = privacy_env
        .build()
        .map_err(|error| ComposerError::ExecutorInput(error.to_string()))?;
    let prove_info = default_prover()
        .prove_with_opts(
            privacy_env,
            PRIVACY_PRESERVING_CIRCUIT_ELF,
            &ProverOpts::succinct(),
        )
        .map_err(|error| ComposerError::PrivacyCircuit(error.to_string()))?;
    let output = prove_info
        .receipt
        .journal
        .decode::<PrivacyPreservingCircuitOutput>()
        .map_err(|error| ComposerError::PrivacyCircuit(error.to_string()))?;
    let proof = LeeProof::from_inner(
        borsh::to_vec(&prove_info.receipt.inner)
            .map_err(|error| ComposerError::PrivacyCircuit(error.to_string()))?,
    );
    Ok((output, proof))
}

fn prove_program(
    program: &Program,
    caller_program_id: Option<ProgramId>,
    pre_states: &[AccountWithMetadata],
    instruction_data: &InstructionData,
    assumption: Option<Receipt>,
) -> Result<Receipt, ComposerError> {
    let mut env = ExecutorEnv::builder();
    env.write(&program.id())
        .and_then(|builder| builder.write(&caller_program_id))
        .and_then(|builder| builder.write(&pre_states.to_vec()))
        .and_then(|builder| builder.write(instruction_data))
        .map_err(|error| ComposerError::ExecutorInput(error.to_string()))?;
    if let Some(receipt) = assumption {
        env.add_assumption(receipt);
    }
    let env = env
        .build()
        .map_err(|error| ComposerError::ExecutorInput(error.to_string()))?;
    default_prover()
        .prove(env, program.elf())
        .map(|info| info.receipt)
        .map_err(|error| ComposerError::ProgramProving(error.to_string()))
}

/// RPC submission, confirmation, and state reconciliation.
#[cfg(feature = "network")]
pub mod network {
    use std::collections::BTreeMap;
    use std::time::{Duration, Instant};

    use common::{transaction::LeeTransaction, HashType};
    use lee::{Account, AccountId, PrivacyPreservingTransaction};
    use lee_core::{account::Nonce, program::ProgramId, BlockId};
    use quorum_gate_core::{
        decode_constitution, decode_proposal, ConstitutionState, ProposalState,
    };
    use sequencer_service_rpc::{RpcClient as _, SequencerClient, SequencerClientBuilder};
    use thiserror::Error;
    use token_core::TokenHolding;

    /// A sequencer-confirmed transaction.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Confirmation {
        /// Deterministic transaction hash.
        pub hash: HashType,
        /// Block containing the transaction.
        pub block: BlockId,
    }

    /// Network client errors with retry-safe transaction hashes.
    #[derive(Debug, Error)]
    pub enum NetworkError {
        /// RPC client construction failed.
        #[error("RPC client: {0}")]
        Client(String),
        /// Submission failed.
        #[error("transaction submission: {0}")]
        Submit(String),
        /// Confirmation timed out; the hash can be queried without resubmitting.
        #[error("transaction {0} was not confirmed before timeout")]
        ConfirmationTimeout(HashType),
        /// Confirmation query failed.
        #[error("transaction confirmation: {0}")]
        Confirmation(String),
        /// State re-read failed.
        #[error("account state: {0}")]
        State(String),
        /// A read-only RPC query failed.
        #[error("RPC query: {0}")]
        Query(String),
    }

    /// Thin client for the LEZ v0.2 sequencer JSON-RPC.
    pub struct NetworkClient {
        client: SequencerClient,
        poll_interval: Duration,
        confirmation_timeout: Duration,
    }

    impl NetworkClient {
        /// Connects to a sequencer URL.
        ///
        /// # Errors
        /// `NetworkError::Client` for an invalid URL or transport setup.
        pub fn connect(url: &str) -> Result<Self, NetworkError> {
            let client = SequencerClientBuilder::default()
                .build(url)
                .map_err(|error| NetworkError::Client(error.to_string()))?;
            Ok(Self {
                client,
                poll_interval: Duration::from_secs(2),
                confirmation_timeout: Duration::from_secs(45),
            })
        }

        /// Overrides polling intervals for tests and operator policy.
        #[must_use]
        pub const fn with_confirmation_policy(
            mut self,
            poll_interval: Duration,
            confirmation_timeout: Duration,
        ) -> Self {
            self.poll_interval = poll_interval;
            self.confirmation_timeout = confirmation_timeout;
            self
        }

        /// Submits once and polls by hash. On timeout callers should reconcile
        /// the returned hash before deciding whether to rebuild a transaction.
        ///
        /// # Errors
        /// Any `NetworkError` variant.
        pub async fn submit_and_confirm(
            &self,
            transaction: PrivacyPreservingTransaction,
        ) -> Result<HashType, NetworkError> {
            self.submit_transaction_and_confirm(LeeTransaction::PrivacyPreserving(transaction))
                .await
        }

        /// Submits any LEZ transaction once and polls its hash to confirmation.
        ///
        /// This is used by deployment and public lifecycle tooling while the
        /// privacy-specific method remains the narrow API for wallet callers.
        ///
        /// # Errors
        /// Any `NetworkError` variant.
        pub async fn submit_transaction_and_confirm(
            &self,
            transaction: LeeTransaction,
        ) -> Result<HashType, NetworkError> {
            self.submit_transaction_and_confirm_with_block(transaction)
                .await
                .map(|confirmation| confirmation.hash)
        }

        /// Submits any LEZ transaction and returns its hash and confirmation block.
        ///
        /// # Errors
        /// Any `NetworkError` variant.
        pub async fn submit_transaction_and_confirm_with_block(
            &self,
            transaction: LeeTransaction,
        ) -> Result<Confirmation, NetworkError> {
            let hash = self.submit_transaction(transaction).await?;
            let block = self.wait_for_transaction(hash).await?;
            Ok(Confirmation { hash, block })
        }

        /// Submits one transaction without polling.
        ///
        /// Callers that persist transaction journals should save the
        /// transaction and its deterministic hash before invoking this method.
        ///
        /// # Errors
        /// `NetworkError::Submit` when the sequencer rejects the request.
        pub async fn submit_transaction(
            &self,
            transaction: LeeTransaction,
        ) -> Result<HashType, NetworkError> {
            self.client
                .send_transaction(transaction)
                .await
                .map_err(|error| NetworkError::Submit(error.to_string()))
        }

        /// Polls an existing hash until it is confirmed and returns its block.
        ///
        /// # Errors
        /// `NetworkError::ConfirmationTimeout` or `NetworkError::Confirmation`.
        pub async fn wait_for_transaction(&self, hash: HashType) -> Result<BlockId, NetworkError> {
            let started = Instant::now();
            loop {
                match self.client.get_transaction(hash).await {
                    Ok(Some((_, block_id))) => return Ok(block_id),
                    Ok(None) if started.elapsed() < self.confirmation_timeout => {
                        tokio::time::sleep(self.poll_interval).await;
                    }
                    Ok(None) => return Err(NetworkError::ConfirmationTimeout(hash)),
                    Err(error) => {
                        return Err(NetworkError::Confirmation(error.to_string()));
                    }
                }
            }
        }

        /// Checks sequencer health.
        ///
        /// # Errors
        /// `NetworkError::Query` when the health RPC fails.
        pub async fn check_health(&self) -> Result<(), NetworkError> {
            self.client
                .check_health()
                .await
                .map_err(|error| NetworkError::Query(error.to_string()))
        }

        /// Returns the latest sequencer block id.
        ///
        /// # Errors
        /// `NetworkError::Query` when the RPC fails.
        pub async fn get_last_block_id(&self) -> Result<BlockId, NetworkError> {
            self.client
                .get_last_block_id()
                .await
                .map_err(|error| NetworkError::Query(error.to_string()))
        }

        /// Reads a transaction and its confirmation block.
        ///
        /// # Errors
        /// `NetworkError::Query` when the RPC fails.
        pub async fn get_transaction(
            &self,
            hash: HashType,
        ) -> Result<Option<(LeeTransaction, BlockId)>, NetworkError> {
            self.client
                .get_transaction(hash)
                .await
                .map_err(|error| NetworkError::Query(error.to_string()))
        }

        /// Finds a transaction in a specific historical block.
        ///
        /// This supports recovery when a sequencer has pruned its transaction
        /// lookup while retaining the block and live program state.
        ///
        /// # Errors
        /// `NetworkError::Query` when the block RPC fails.
        pub async fn get_transaction_in_block(
            &self,
            hash: HashType,
            block_id: BlockId,
        ) -> Result<Option<LeeTransaction>, NetworkError> {
            let block = self
                .client
                .get_block(block_id)
                .await
                .map_err(|error| NetworkError::Query(error.to_string()))?;
            Ok(block.and_then(|block| {
                block
                    .body
                    .transactions
                    .into_iter()
                    .find(|transaction| transaction.hash() == hash)
            }))
        }

        /// Reads current nonces for public signer accounts.
        ///
        /// # Errors
        /// `NetworkError::Query` when the RPC fails.
        pub async fn get_account_nonces(
            &self,
            account_ids: Vec<AccountId>,
        ) -> Result<Vec<Nonce>, NetworkError> {
            self.client
                .get_accounts_nonces(account_ids)
                .await
                .map_err(|error| NetworkError::Query(error.to_string()))
        }

        /// Reads the native balance of a public account.
        ///
        /// # Errors
        /// `NetworkError::Query` when the RPC fails.
        pub async fn get_account_balance(
            &self,
            account_id: AccountId,
        ) -> Result<u128, NetworkError> {
            self.client
                .get_account_balance(account_id)
                .await
                .map_err(|error| NetworkError::Query(error.to_string()))
        }

        /// Returns all named built-in program ids.
        ///
        /// # Errors
        /// `NetworkError::Query` when the RPC fails.
        pub async fn get_program_ids(&self) -> Result<BTreeMap<String, ProgramId>, NetworkError> {
            self.client
                .get_program_ids()
                .await
                .map_err(|error| NetworkError::Query(error.to_string()))
        }

        /// Returns the active sequencer channel id.
        ///
        /// # Errors
        /// `NetworkError::Query` when the RPC fails.
        pub async fn get_channel_id(&self) -> Result<String, NetworkError> {
            self.client
                .get_channel_id()
                .await
                .map(|channel_id| channel_id.to_string())
                .map_err(|error| NetworkError::Query(error.to_string()))
        }

        /// Re-reads an account after transaction confirmation.
        ///
        /// # Errors
        /// `NetworkError::State` when the account cannot be fetched.
        pub async fn get_account(&self, account_id: AccountId) -> Result<Account, NetworkError> {
            self.client
                .get_account(account_id)
                .await
                .map_err(|error| NetworkError::State(error.to_string()))
        }

        /// Reads and decodes a Quorum constitution account.
        ///
        /// # Errors
        /// `NetworkError::State` when the account cannot be fetched or decoded.
        pub async fn get_constitution(
            &self,
            account_id: AccountId,
        ) -> Result<ConstitutionState, NetworkError> {
            let account = self.get_account(account_id).await?;
            decode_constitution(&account.data)
                .map_err(|error| NetworkError::State(error.to_string()))
        }

        /// Reads and decodes a Quorum proposal account.
        ///
        /// # Errors
        /// `NetworkError::State` when the account cannot be fetched or decoded.
        pub async fn get_proposal(
            &self,
            account_id: AccountId,
        ) -> Result<ProposalState, NetworkError> {
            let account = self.get_account(account_id).await?;
            decode_proposal(&account.data).map_err(|error| NetworkError::State(error.to_string()))
        }

        /// Reads and decodes a token holding account.
        ///
        /// # Errors
        /// `NetworkError::State` when the account cannot be fetched or decoded.
        pub async fn get_token_holding(
            &self,
            account_id: AccountId,
        ) -> Result<TokenHolding, NetworkError> {
            let account = self.get_account(account_id).await?;
            TokenHolding::try_from(&account.data)
                .map_err(|error| NetworkError::State(error.to_string()))
        }
    }

    #[cfg(test)]
    mod tests {
        use std::sync::Arc;

        use jsonrpsee::{server::ServerHandle, types::ErrorObjectOwned, RpcModule};

        use super::*;
        use crate::lifecycle;

        #[derive(Clone)]
        struct MockState {
            transaction: LeeTransaction,
            reject_submission: bool,
            confirm: bool,
        }

        async fn mock_server(state: MockState) -> (String, ServerHandle) {
            let server = jsonrpsee::server::Server::builder()
                .build("127.0.0.1:0")
                .await
                .unwrap();
            let address = server.local_addr().unwrap();
            let mut module = RpcModule::new(Arc::new(state));
            module
                .register_method::<Result<HashType, ErrorObjectOwned>, _>(
                    "sendTransaction",
                    |params, state, _| {
                        let submitted: LeeTransaction = params.one()?;
                        if state.reject_submission {
                            return Err(ErrorObjectOwned::owned(
                                -32_000,
                                "submission rejected",
                                None::<()>,
                            ));
                        }
                        if submitted != state.transaction {
                            return Err(ErrorObjectOwned::owned(
                                -32_001,
                                "unexpected transaction",
                                None::<()>,
                            ));
                        }
                        Ok(submitted.hash())
                    },
                )
                .unwrap();
            module
                .register_method::<Result<Option<(LeeTransaction, BlockId)>, ErrorObjectOwned>, _>(
                    "getTransaction",
                    |params, state, _| {
                        let hash: HashType = params.one()?;
                        if hash != state.transaction.hash() {
                            return Err(ErrorObjectOwned::owned(
                                -32_002,
                                "unexpected hash",
                                None::<()>,
                            ));
                        }
                        Ok(state.confirm.then(|| (state.transaction.clone(), 41)))
                    },
                )
                .unwrap();
            let handle = server.start(module);
            (format!("http://{address}"), handle)
        }

        #[tokio::test]
        async fn submit_returns_typed_hash_and_block() {
            let transaction = lifecycle::deploy_gate();
            let (url, handle) = mock_server(MockState {
                transaction: transaction.clone(),
                reject_submission: false,
                confirm: true,
            })
            .await;
            let client = NetworkClient::connect(&url).unwrap();
            let confirmation = client
                .submit_transaction_and_confirm_with_block(transaction.clone())
                .await
                .unwrap();
            assert_eq!(confirmation.hash, transaction.hash());
            assert_eq!(confirmation.block, 41);
            handle.stop().unwrap();
        }

        #[tokio::test]
        async fn submission_rejection_is_not_reported_as_confirmation() {
            let transaction = lifecycle::deploy_gate();
            let (url, handle) = mock_server(MockState {
                transaction: transaction.clone(),
                reject_submission: true,
                confirm: false,
            })
            .await;
            let client = NetworkClient::connect(&url).unwrap();
            assert!(matches!(
                client.submit_transaction(transaction).await,
                Err(NetworkError::Submit(_))
            ));
            handle.stop().unwrap();
        }

        #[tokio::test]
        async fn confirmation_timeout_preserves_the_queryable_hash() {
            let transaction = lifecycle::deploy_gate();
            let hash = transaction.hash();
            let (url, handle) = mock_server(MockState {
                transaction: transaction.clone(),
                reject_submission: false,
                confirm: false,
            })
            .await;
            let client = NetworkClient::connect(&url)
                .unwrap()
                .with_confirmation_policy(Duration::from_millis(1), Duration::from_millis(4));
            assert!(matches!(
                client.submit_transaction_and_confirm(transaction).await,
                Err(NetworkError::ConfirmationTimeout(value)) if value == hash
            ));
            handle.stop().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use lee_core::{
        account::{Account, AccountId, AccountWithMetadata},
        encryption::ViewingPublicKey,
        program::ProgramOutput,
    };
    use quorum_circuit::{
        evaluate, ActionData, MemberApprovalWitness, ThresholdJournal, ThresholdWitness,
    };
    use quorum_core::{merkle::MemberTree, nullifier::member_commitment_for_credential};
    use quorum_gate_core::{
        decode_proposal, encode_constitution, encode_proposal, ConstitutionState, ProposalState,
        TierPolicy,
    };
    use quorum_gate_methods::{QUORUM_GATE_ELF, QUORUM_GATE_ID};
    use quorum_prover::QuorumProof;
    use quorum_threshold_methods::THRESHOLD_ELF;
    use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};

    fn viewing_public_key(key_byte: u8) -> ViewingPublicKey {
        ViewingPublicKey::from_seed(&[key_byte; 32], &[key_byte + 1; 32])
    }

    fn viewing_public_key_bytes(key_byte: u8) -> [u8; quorum_core::VIEWING_PUBLIC_KEY_LEN] {
        viewing_public_key(key_byte)
            .to_bytes()
            .try_into()
            .expect("official viewing public key length")
    }

    fn threshold_witness() -> (ThresholdWitness, Vec<[u8; 32]>) {
        let secrets = [[1_u8; 32], [2_u8; 32], [3_u8; 32]];
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
        (
            ThresholdWitness {
                member_root: tree.root(),
                required_threshold: 2,
                approvals: vec![
                    approval_for(secrets[0], viewing_public_keys[0]),
                    approval_for(secrets[1], viewing_public_keys[1]),
                ],
                action: ActionData::Transfer {
                    recipient: [9_u8; 32],
                    amount: 500,
                    tier_id: 1,
                    tier_max_amount: 1_000,
                },
                proposal_id: 7,
                constitution_version: 1,
            },
            secrets
                .iter()
                .zip(&viewing_public_keys)
                .take(2)
                .map(|(secret, viewing_public_key)| {
                    lez_compat::private_account_id(secret, viewing_public_key, 0)
                })
                .collect(),
        )
    }

    fn dev_proof(witness: &ThresholdWitness) -> QuorumProof {
        let expected = evaluate(witness).expect("valid witness");
        let env = ExecutorEnv::builder()
            .write(witness)
            .expect("threshold input")
            .build()
            .expect("threshold environment");
        let info = default_prover()
            .prove_with_opts(env, THRESHOLD_ELF, &ProverOpts::succinct())
            .expect("threshold proof");
        let journal = info
            .receipt
            .journal
            .decode::<ThresholdJournal>()
            .expect("threshold journal");
        assert_eq!(journal, expected);
        QuorumProof {
            journal,
            receipt: bincode::serialize(&info.receipt).expect("receipt serialization"),
        }
    }

    fn account(
        account_id: [u8; 32],
        owner: [u32; 8],
        data: Vec<u8>,
        authorized: bool,
    ) -> AccountWithMetadata {
        AccountWithMetadata::new(
            Account {
                program_owner: owner,
                data: data.try_into().expect("account data fits"),
                ..Account::default()
            },
            authorized,
            AccountId::new(account_id),
        )
    }

    fn gate_accounts(
        witness: &ThresholdWitness,
        credential_ids: &[[u8; 32]],
    ) -> Vec<AccountWithMetadata> {
        let multisig_id = [21_u8; 32];
        let constitution = ConstitutionState::new(
            multisig_id,
            2,
            3,
            witness.member_root,
            vec![TierPolicy {
                id: 1,
                threshold: 2,
                max_amount: 1_000,
            }],
        )
        .expect("constitution");
        let proposal = ProposalState::new(
            multisig_id,
            witness.proposal_id,
            witness.constitution_version,
            witness.required_threshold,
            witness.action.clone(),
        );
        let mut accounts = vec![
            account(
                multisig_id,
                QUORUM_GATE_ID,
                encode_constitution(&constitution).expect("constitution encoding"),
                false,
            ),
            account(
                [22_u8; 32],
                QUORUM_GATE_ID,
                encode_proposal(&proposal).expect("proposal encoding"),
                false,
            ),
        ];
        accounts.extend(
            credential_ids
                .iter()
                .map(|account_id| account(*account_id, [0_u32; 8], Vec::new(), true)),
        );
        accounts
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

    fn fixture_request(
        witness: &ThresholdWitness,
        credential_ids: &[[u8; 32]],
    ) -> PrivateApprovalRequest {
        let pre_states = gate_accounts(witness, credential_ids);
        let public_account_ids = pre_states[..2]
            .iter()
            .map(|account| account.account_id)
            .collect::<Vec<_>>();
        let program = Program::new(Cow::Borrowed(QUORUM_GATE_ELF)).expect("gate program");
        PrivateApprovalRequest {
            programs: program.into(),
            pre_states,
            account_identities: vec![
                InputAccountIdentity::Public,
                InputAccountIdentity::Public,
                private_init_identity([1_u8; 32], 31),
                private_init_identity([2_u8; 32], 41),
            ],
            dummy_inputs: vec![],
            public_account_ids,
            public_nonces: Vec::new(),
            public_signers: Vec::new(),
            proposal_id: witness.proposal_id,
        }
    }

    fn compose_fixture(
        witness: &ThresholdWitness,
        credential_ids: &[[u8; 32]],
        proof: &QuorumProof,
    ) -> ComposedApproval {
        compose_private_approval(fixture_request(witness, credential_ids), proof)
            .expect("private approval transaction")
    }

    #[test]
    fn prepared_journal_is_byte_identical_to_threshold_journal() {
        let (witness, credential_ids) = threshold_witness();
        let proof = dev_proof(&witness);
        let prepared = prepare_approval(&proof, witness.proposal_id, credential_ids)
            .expect("prepared approval");
        let QuorumInstruction::Approve { claim, .. } = prepared.instruction else {
            panic!("expected approve instruction");
        };
        assert_eq!(
            risc0_zkvm::serde::to_vec(&proof.journal).expect("threshold words"),
            risc0_zkvm::serde::to_vec(&claim.journal).expect("gate words")
        );
    }

    #[test]
    fn gate_accepts_matching_threshold_receipt_assumption() {
        let (witness, credential_ids) = threshold_witness();
        let proof = dev_proof(&witness);
        let prepared =
            prepare_approval(&proof, witness.proposal_id, credential_ids.clone()).expect("prepare");
        let instruction_data =
            Program::serialize_instruction(&prepared.instruction).expect("instruction");
        let program = Program::new(Cow::Borrowed(QUORUM_GATE_ELF)).expect("gate program");
        let receipt = prove_program(
            &program,
            None,
            &gate_accounts(&witness, &credential_ids),
            &instruction_data,
            Some(prepared.receipt),
        )
        .expect("gate receipt");
        let output = receipt
            .journal
            .decode::<ProgramOutput>()
            .expect("gate output");
        let proposal =
            decode_proposal(&output.post_states[1].account().data).expect("updated proposal state");
        assert_eq!(proposal.nullifiers.len(), 2);
        assert!(proposal.threshold_met());
        assert_eq!(output.post_states.len(), 4);
    }

    #[test]
    fn gate_rejects_missing_receipt_assumption() {
        let (witness, credential_ids) = threshold_witness();
        let proof = dev_proof(&witness);
        let prepared =
            prepare_approval(&proof, witness.proposal_id, credential_ids.clone()).expect("prepare");
        let instruction_data =
            Program::serialize_instruction(&prepared.instruction).expect("instruction");
        let program = Program::new(Cow::Borrowed(QUORUM_GATE_ELF)).expect("gate program");
        assert!(prove_program(
            &program,
            None,
            &gate_accounts(&witness, &credential_ids),
            &instruction_data,
            None,
        )
        .is_err());
    }

    #[test]
    fn composes_private_transaction_with_hidden_credentials() {
        let (witness, credential_ids) = threshold_witness();
        let proof = dev_proof(&witness);
        let composed = compose_fixture(&witness, &credential_ids, &proof);

        let message = composed.transaction.message();
        let public_account_ids = message.public_account_ids();
        assert_eq!(public_account_ids.len(), 2);
        assert_eq!(message.public_actions.len(), 2);
        assert_eq!(message.private_actions.len(), 2);
        assert_eq!(message.commitments().len(), 2);
        assert_eq!(message.nullifiers().len(), 2);
        assert_eq!(composed.credential_account_ids, credential_ids);
        for credential_id in &credential_ids {
            assert!(!public_account_ids
                .iter()
                .any(|account_id| account_id.value() == credential_id));
        }
    }

    #[test]
    fn rejects_requested_public_accounts_that_differ_from_circuit_actions() {
        let (witness, credential_ids) = threshold_witness();
        let proof = dev_proof(&witness);
        let mut request = fixture_request(&witness, &credential_ids);
        request.public_account_ids.reverse();

        assert!(matches!(
            compose_private_approval(request, &proof),
            Err(ComposerError::PrivateMessage(_))
        ));
    }

    #[test]
    #[ignore = "requires a real artifact produced with QUORUM_PROOF_OUTPUT"]
    fn composes_stored_real_threshold_receipt() {
        let path = std::env::var("QUORUM_REAL_PROOF").expect("QUORUM_REAL_PROOF path");
        let bytes = std::fs::read(path).expect("real proof artifact");
        let proof: QuorumProof = serde_json::from_slice(&bytes).expect("real proof JSON");
        let (witness, credential_ids) = threshold_witness();
        assert_eq!(proof.journal, evaluate(&witness).expect("fixture journal"));
        let composed = compose_fixture(&witness, &credential_ids, &proof);
        assert_eq!(composed.transaction.message().commitments().len(), 2);
    }

    #[test]
    fn preflight_rejects_credential_and_receipt_tampering() {
        let (witness, credential_ids) = threshold_witness();
        let proof = dev_proof(&witness);
        assert!(matches!(
            prepare_approval(&proof, witness.proposal_id, vec![[99_u8; 32]; 2]),
            Err(ComposerError::CredentialMismatch)
        ));

        let mut malformed = proof;
        malformed.receipt.truncate(8);
        assert!(matches!(
            prepare_approval(&malformed, witness.proposal_id, credential_ids),
            Err(ComposerError::ThresholdProof(_))
        ));
    }

    #[test]
    fn threshold_receipt_is_bound_to_the_pinned_image() {
        let (witness, _) = threshold_witness();
        let proof = dev_proof(&witness);
        let receipt = decode_receipt(&proof.receipt).expect("threshold receipt");
        assert!(receipt.verify([0_u32; 8]).is_err());
    }
}
