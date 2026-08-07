//! Deterministic builders for the public Quorum treasury lifecycle.

use std::borrow::Cow;

use common::transaction::LeeTransaction;
use lee::{
    program::Program,
    program_deployment_transaction::Message as DeploymentMessage,
    public_transaction::{Message as PublicMessage, WitnessSet as PublicWitnessSet},
    AccountId, PrivateKey, ProgramDeploymentTransaction, PublicKey, PublicTransaction,
};
use lee_core::{
    account::Nonce,
    program::{InstructionData, PdaSeed, ProgramId},
};
use quorum_circuit::ActionData;
use quorum_gate_core::{vault_pda_seed, QuorumInstruction, TierPolicy};
use quorum_gate_methods::{QUORUM_GATE_ELF, QUORUM_GATE_ID};
use token_core::Instruction as TokenInstruction;

use crate::ComposerError;

/// Seed material for the five public demo accounts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleSeeds {
    /// Constitution account signer seed.
    pub multisig: [u8; 32],
    /// Token definition account signer seed.
    pub definition: [u8; 32],
    /// Initial token supply account signer seed.
    pub supply: [u8; 32],
    /// Transfer recipient account signer seed.
    pub recipient: [u8; 32],
    /// Proposal account signer seed.
    pub proposal: [u8; 32],
}

/// One public account and its signing key.
pub struct PublicAccount {
    /// Public signing key.
    pub key: PrivateKey,
    /// Derived account id.
    pub id: AccountId,
}

impl PublicAccount {
    fn from_seed(seed: [u8; 32]) -> Result<Self, ComposerError> {
        let key = PrivateKey::try_new(seed)
            .map_err(|error| ComposerError::AccountKey(error.to_string()))?;
        let id = AccountId::from(&PublicKey::new_from_private_key(&key));
        Ok(Self { key, id })
    }
}

/// Accounts used by one public treasury lifecycle.
pub struct LifecycleAccounts {
    /// Constitution account.
    pub multisig: PublicAccount,
    /// Token definition account.
    pub definition: PublicAccount,
    /// Initial token supply account.
    pub supply: PublicAccount,
    /// Transfer recipient account.
    pub recipient: PublicAccount,
    /// Proposal account.
    pub proposal: PublicAccount,
    /// Program-derived treasury vault.
    pub vault_id: AccountId,
}

impl LifecycleAccounts {
    /// Derives lifecycle accounts from private seed material.
    ///
    /// # Errors
    /// `ComposerError::AccountKey` if a seed is not a valid LEZ key.
    pub fn from_seeds(seeds: &LifecycleSeeds) -> Result<Self, ComposerError> {
        let multisig = PublicAccount::from_seed(seeds.multisig)?;
        let vault_id = AccountId::for_public_pda(
            &QUORUM_GATE_ID,
            &PdaSeed::new(vault_pda_seed(multisig.id.value())),
        );
        Ok(Self {
            multisig,
            definition: PublicAccount::from_seed(seeds.definition)?,
            supply: PublicAccount::from_seed(seeds.supply)?,
            recipient: PublicAccount::from_seed(seeds.recipient)?,
            proposal: PublicAccount::from_seed(seeds.proposal)?,
            vault_id,
        })
    }
}

fn instruction<T: serde::Serialize>(value: T) -> Result<InstructionData, ComposerError> {
    Program::serialize_instruction(value)
        .map_err(|error| ComposerError::LifecycleInstruction(error.to_string()))
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

/// Returns the pinned gate program and verifies its id.
///
/// # Errors
/// `ComposerError::GateProgram` if the embedded ELF is malformed or its id has
/// changed.
pub fn gate_program() -> Result<Program, ComposerError> {
    let program = Program::new(Cow::Borrowed(QUORUM_GATE_ELF))
        .map_err(|error| ComposerError::GateProgram(error.to_string()))?;
    if program.id() != QUORUM_GATE_ID {
        return Err(ComposerError::GateProgram(
            "embedded gate id does not match the pinned id".to_owned(),
        ));
    }
    Ok(program)
}

/// Builds the pinned gate deployment transaction.
#[must_use]
pub fn deploy_gate() -> LeeTransaction {
    LeeTransaction::ProgramDeployment(ProgramDeploymentTransaction::new(DeploymentMessage::new(
        QUORUM_GATE_ELF.to_vec(),
    )))
}

/// Builds constitution initialization.
///
/// # Errors
/// `ComposerError::LifecycleInstruction` if instruction encoding fails.
pub fn initialize_constitution(
    accounts: &LifecycleAccounts,
    nonce: Nonce,
    threshold: u8,
    member_count: u8,
    member_root: [u8; 32],
    tiers: Vec<TierPolicy>,
) -> Result<LeeTransaction, ComposerError> {
    Ok(public_transaction(
        QUORUM_GATE_ID,
        vec![accounts.multisig.id],
        vec![nonce],
        &[&accounts.multisig.key],
        instruction(QuorumInstruction::Initialize {
            threshold,
            member_count,
            member_root,
            tiers,
        })?,
    ))
}

/// Builds fungible token definition and initial supply creation.
///
/// # Errors
/// `ComposerError::LifecycleInstruction` if instruction encoding fails.
pub fn create_token(
    accounts: &LifecycleAccounts,
    definition_nonce: Nonce,
    supply_nonce: Nonce,
    name: String,
    total_supply: u128,
) -> Result<LeeTransaction, ComposerError> {
    Ok(public_transaction(
        programs::token().id(),
        vec![accounts.definition.id, accounts.supply.id],
        vec![definition_nonce, supply_nonce],
        &[&accounts.definition.key, &accounts.supply.key],
        instruction(TokenInstruction::NewFungibleDefinition { name, total_supply })?,
    ))
}

/// Builds recipient token-account initialization.
///
/// # Errors
/// `ComposerError::LifecycleInstruction` if instruction encoding fails.
pub fn initialize_recipient(
    accounts: &LifecycleAccounts,
    recipient_nonce: Nonce,
) -> Result<LeeTransaction, ComposerError> {
    Ok(public_transaction(
        programs::token().id(),
        vec![accounts.definition.id, accounts.recipient.id],
        vec![recipient_nonce],
        &[&accounts.recipient.key],
        instruction(TokenInstruction::InitializeAccount)?,
    ))
}

/// Builds program-derived treasury vault initialization.
///
/// # Errors
/// `ComposerError::LifecycleInstruction` if instruction encoding fails.
pub fn initialize_vault(
    accounts: &LifecycleAccounts,
    multisig_nonce: Nonce,
) -> Result<LeeTransaction, ComposerError> {
    Ok(public_transaction(
        QUORUM_GATE_ID,
        vec![
            accounts.multisig.id,
            accounts.definition.id,
            accounts.vault_id,
        ],
        vec![multisig_nonce],
        &[&accounts.multisig.key],
        instruction(QuorumInstruction::InitializeVault)?,
    ))
}

/// Builds treasury vault funding from the initial supply account.
///
/// # Errors
/// `ComposerError::LifecycleInstruction` if instruction encoding fails.
pub fn fund_vault(
    accounts: &LifecycleAccounts,
    supply_nonce: Nonce,
    amount: u128,
) -> Result<LeeTransaction, ComposerError> {
    Ok(public_transaction(
        programs::token().id(),
        vec![accounts.supply.id, accounts.vault_id],
        vec![supply_nonce],
        &[&accounts.supply.key],
        instruction(TokenInstruction::Transfer {
            amount_to_transfer: amount,
        })?,
    ))
}

/// Builds a proposal transaction.
///
/// # Errors
/// `ComposerError::LifecycleInstruction` if instruction encoding fails.
pub fn propose(
    accounts: &LifecycleAccounts,
    proposal_nonce: Nonce,
    action: ActionData,
) -> Result<LeeTransaction, ComposerError> {
    Ok(public_transaction(
        QUORUM_GATE_ID,
        vec![accounts.multisig.id, accounts.proposal.id],
        vec![proposal_nonce],
        &[&accounts.proposal.key],
        instruction(QuorumInstruction::Propose { action })?,
    ))
}

/// Builds one-shot proposal execution.
///
/// # Errors
/// `ComposerError::LifecycleInstruction` if instruction encoding fails.
pub fn execute(
    accounts: &LifecycleAccounts,
    proposal_id: u64,
) -> Result<LeeTransaction, ComposerError> {
    Ok(public_transaction(
        QUORUM_GATE_ID,
        vec![
            accounts.multisig.id,
            accounts.proposal.id,
            accounts.vault_id,
            accounts.recipient.id,
        ],
        Vec::new(),
        &[],
        instruction(QuorumInstruction::Execute { proposal_id })?,
    ))
}

#[cfg(test)]
mod tests {
    use common::HashType;

    use super::*;

    fn accounts() -> LifecycleAccounts {
        LifecycleAccounts::from_seeds(&LifecycleSeeds {
            multisig: [91; 32],
            definition: [92; 32],
            supply: [93; 32],
            recipient: [94; 32],
            proposal: [95; 32],
        })
        .unwrap()
    }

    #[test]
    fn pinned_deployment_hash_matches_public_evidence() {
        let expected = "4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43"
            .parse::<HashType>()
            .unwrap();
        assert_eq!(deploy_gate().hash(), expected);
        assert_eq!(gate_program().unwrap().id(), QUORUM_GATE_ID);
    }

    #[test]
    fn public_builder_binds_the_expected_account() {
        let accounts = accounts();
        let transaction = initialize_constitution(
            &accounts,
            Nonce(0),
            2,
            3,
            [7; 32],
            vec![TierPolicy {
                id: 1,
                threshold: 2,
                max_amount: 1_000,
            }],
        )
        .unwrap();
        assert_eq!(
            transaction.affected_public_account_ids(),
            vec![accounts.multisig.id]
        );
    }

    #[test]
    fn every_public_lifecycle_transaction_serializes_and_roundtrips() {
        let accounts = accounts();
        let action = ActionData::Transfer {
            recipient: *accounts.recipient.id.value(),
            amount: 250,
            tier_id: 1,
            tier_max_amount: 1_000,
        };
        let transactions = [
            deploy_gate(),
            initialize_constitution(
                &accounts,
                Nonce(0),
                2,
                3,
                [7; 32],
                vec![TierPolicy {
                    id: 1,
                    threshold: 2,
                    max_amount: 1_000,
                }],
            )
            .unwrap(),
            create_token(
                &accounts,
                Nonce(0),
                Nonce(0),
                "QUORUM-DEMO".to_owned(),
                1_000,
            )
            .unwrap(),
            initialize_recipient(&accounts, Nonce(0)).unwrap(),
            initialize_vault(&accounts, Nonce(1)).unwrap(),
            fund_vault(&accounts, Nonce(1), 750).unwrap(),
            propose(&accounts, Nonce(0), action).unwrap(),
            execute(&accounts, 0).unwrap(),
        ];

        for transaction in transactions {
            let expected_hash = transaction.hash();
            let json = serde_json::to_vec(&transaction).unwrap();
            let decoded: LeeTransaction = serde_json::from_slice(&json).unwrap();
            assert_eq!(decoded, transaction);
            assert_eq!(decoded.hash(), expected_hash);
        }
    }
}
