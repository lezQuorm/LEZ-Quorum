use std::time::Instant;

use anyhow::{bail, Context as _, Result};
use lee::{program::Program, Account, AccountId};
use lee_core::{
    account::{AccountWithMetadata, Data},
    encryption::ViewingPublicKey,
    program::{PdaSeed, ProgramId},
};
use quorum_circuit::{
    evaluate, ActionData, MemberApprovalWitness, ThresholdJournal, ThresholdWitness,
};
use quorum_composer::lifecycle;
use quorum_core::{
    merkle::MemberTree,
    nullifier::{derive_nullifier, member_commitment_for_credential},
};
use quorum_gate_core::{
    encode_constitution, encode_proposal, vault_pda_seed, ConstitutionState,
    OnChainThresholdJournal, ProposalState, QuorumInstruction, ThresholdClaim, TierPolicy,
};
use quorum_gate_methods::QUORUM_GATE_ID;
use quorum_threshold_methods::THRESHOLD_ELF;
use risc0_zkvm::{default_executor, default_prover, ExecutorEnv, ProverOpts, Receipt};
use serde::Serialize;
use token_core::{Instruction as TokenInstruction, TokenDefinition, TokenHolding};

const TRANSFER_AMOUNT: u64 = 250;
const TIER_MAX: u64 = 750;

#[derive(Clone)]
struct Case {
    operation: &'static str,
    program: Program,
    pre_states: Vec<AccountWithMetadata>,
    instruction: lee_core::program::InstructionData,
    assumption: Option<Receipt>,
}

impl Case {
    fn new<I: Serialize>(
        operation: &'static str,
        program: Program,
        pre_states: Vec<AccountWithMetadata>,
        instruction: &I,
        assumption: Option<Receipt>,
    ) -> Result<Self> {
        Ok(Self {
            operation,
            program,
            pre_states,
            instruction: Program::serialize_instruction(instruction)
                .context("serialize instruction")?,
            assumption,
        })
    }

    fn gate(
        operation: &'static str,
        pre_states: Vec<AccountWithMetadata>,
        instruction: &QuorumInstruction,
        assumption: Option<Receipt>,
    ) -> Result<Self> {
        Self::new(
            operation,
            lifecycle::gate_program()?,
            pre_states,
            instruction,
            assumption,
        )
    }

    fn run(&self) -> Result<(u64, usize, f64)> {
        let caller_program_id: Option<ProgramId> = None;
        let mut best_ms = f64::MAX;
        let mut final_cycles = 0;
        let mut final_segments = 0;

        for iteration in 0..4 {
            let mut env = ExecutorEnv::builder();
            env.write(&self.program.id())?
                .write(&caller_program_id)?
                .write(&self.pre_states)?
                .write(&self.instruction)?;
            if let Some(receipt) = &self.assumption {
                env.add_assumption(receipt.clone());
            }
            let env = env.build()?;
            let started = Instant::now();
            let info = default_executor().execute(env, self.program.elf())?;
            let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
            if iteration > 0 {
                best_ms = best_ms.min(elapsed_ms);
            }
            final_cycles = info.cycles();
            final_segments = info.segments.len();
        }

        Ok((final_cycles, final_segments, best_ms))
    }
}

struct Fixture {
    multisig_id: AccountId,
    proposal_id: AccountId,
    vault_id: AccountId,
    recipient_id: AccountId,
    definition_id: AccountId,
    constitution: ConstitutionState,
    transfer: ProposalState,
    approve_claim: ThresholdClaim,
    approve_receipt: Receipt,
    credential_id: AccountId,
}

fn viewing_public_key(key: u8) -> ViewingPublicKey {
    ViewingPublicKey::from_seed(&[key; 32], &[key + 1; 32])
}

fn viewing_public_key_bytes(key: u8) -> [u8; quorum_core::VIEWING_PUBLIC_KEY_LEN] {
    viewing_public_key(key)
        .to_bytes()
        .try_into()
        .expect("official LEZ viewing public key length")
}

fn approval_fixture() -> Result<Fixture> {
    let multisig_id = AccountId::new([11; 32]);
    let proposal_id = AccountId::new([12; 32]);
    let recipient_id = AccountId::new([13; 32]);
    let definition_id = AccountId::new([14; 32]);
    let secrets = [[21; 32], [22; 32], [23; 32]];
    let viewing_keys = [
        viewing_public_key_bytes(31),
        viewing_public_key_bytes(41),
        viewing_public_key_bytes(51),
    ];
    let commitments = secrets
        .iter()
        .zip(&viewing_keys)
        .map(|(secret, key)| member_commitment_for_credential(secret, key, 0))
        .collect::<Vec<_>>();
    let tree = MemberTree::new(&commitments);
    let member_path = tree.proof_for(&commitments[0]).context("member path")?;
    let action = ActionData::Transfer {
        recipient: *recipient_id.value(),
        amount: TRANSFER_AMOUNT,
        tier_id: 1,
        tier_max_amount: TIER_MAX,
    };
    let witness = ThresholdWitness {
        member_root: tree.root(),
        required_threshold: 1,
        approvals: vec![MemberApprovalWitness {
            member_secret: secrets[0],
            viewing_public_key: viewing_keys[0],
            account_identifier: 0,
            leaf_index: member_path.leaf_index,
            siblings: member_path.siblings,
        }],
        action: action.clone(),
        proposal_id: 0,
        constitution_version: 1,
    };
    let expected = evaluate(&witness).context("approval witness")?;
    let env = ExecutorEnv::builder().write(&witness)?.build()?;
    let proof = default_prover()
        .prove_with_opts(env, THRESHOLD_ELF, &ProverOpts::succinct())
        .context("development threshold receipt")?;
    let journal = proof.receipt.journal.decode::<ThresholdJournal>()?;
    if journal != expected {
        bail!("threshold journal mismatch");
    }

    let constitution = ConstitutionState::new(
        *multisig_id.value(),
        2,
        3,
        tree.root(),
        vec![TierPolicy {
            id: 1,
            threshold: 2,
            max_amount: TIER_MAX,
        }],
    )?;
    let transfer = ProposalState::new(*multisig_id.value(), 0, 1, 2, action);
    let credential_id = AccountId::new(lez_compat::private_account_id(
        &secrets[0],
        &viewing_keys[0],
        0,
    ));
    let vault_id = AccountId::for_public_pda(
        &QUORUM_GATE_ID,
        &PdaSeed::new(vault_pda_seed(multisig_id.value())),
    );

    Ok(Fixture {
        multisig_id,
        proposal_id,
        vault_id,
        recipient_id,
        definition_id,
        constitution,
        transfer,
        approve_claim: ThresholdClaim {
            journal: OnChainThresholdJournal::from(&journal),
        },
        approve_receipt: proof.receipt,
        credential_id,
    })
}

fn account(
    id: AccountId,
    owner: ProgramId,
    data: Vec<u8>,
    authorized: bool,
) -> Result<AccountWithMetadata> {
    Ok(AccountWithMetadata::new(
        Account {
            program_owner: owner,
            data: Data::try_from(data).context("account data")?,
            ..Account::default()
        },
        authorized,
        id,
    ))
}

fn empty_account(id: AccountId, authorized: bool) -> AccountWithMetadata {
    AccountWithMetadata::new(Account::default(), authorized, id)
}

fn constitution_account(fixture: &Fixture, authorized: bool) -> Result<AccountWithMetadata> {
    account(
        fixture.multisig_id,
        QUORUM_GATE_ID,
        encode_constitution(&fixture.constitution)?,
        authorized,
    )
}

fn proposal_account(fixture: &Fixture, proposal: &ProposalState) -> Result<AccountWithMetadata> {
    account(
        fixture.proposal_id,
        QUORUM_GATE_ID,
        encode_proposal(proposal)?,
        false,
    )
}

fn token_account(
    id: AccountId,
    definition_id: AccountId,
    balance: u128,
    authorized: bool,
) -> Result<AccountWithMetadata> {
    let holding = TokenHolding::Fungible {
        definition_id,
        balance,
    };
    account(
        id,
        programs::token().id(),
        borsh::to_vec(&holding)?,
        authorized,
    )
}

fn token_definition_account(fixture: &Fixture) -> Result<AccountWithMetadata> {
    let definition = TokenDefinition::Fungible {
        name: "QUORUM-BENCH".to_owned(),
        total_supply: 1_000,
        metadata_id: None,
    };
    account(
        fixture.definition_id,
        programs::token().id(),
        borsh::to_vec(&definition)?,
        false,
    )
}

fn approved(mut proposal: ProposalState) -> Result<ProposalState> {
    proposal.add_nullifier(derive_nullifier(&[21; 32], 0, 1))?;
    proposal.add_nullifier(derive_nullifier(&[22; 32], 0, 1))?;
    Ok(proposal)
}

fn propose_case(fixture: &Fixture, operation: &'static str, action: ActionData) -> Result<Case> {
    Case::gate(
        operation,
        vec![
            constitution_account(fixture, false)?,
            empty_account(fixture.proposal_id, true),
        ],
        &QuorumInstruction::Propose { action },
        None,
    )
}

fn execute_case(
    fixture: &Fixture,
    operation: &'static str,
    proposal: ProposalState,
) -> Result<Case> {
    Case::gate(
        operation,
        vec![
            constitution_account(fixture, false)?,
            proposal_account(fixture, &approved(proposal)?)?,
            token_account(fixture.vault_id, fixture.definition_id, 750, false)?,
            token_account(fixture.recipient_id, fixture.definition_id, 0, false)?,
        ],
        &QuorumInstruction::Execute { proposal_id: 0 },
        None,
    )
}

fn cases(fixture: &Fixture) -> Result<Vec<Case>> {
    let rotation = ActionData::RotateMembers {
        new_member_root: [44; 32],
        new_member_count: 3,
    };
    let threshold = ActionData::ChangeThreshold { new_threshold: 1 };
    let rotation_proposal =
        ProposalState::new(*fixture.multisig_id.value(), 0, 1, 2, rotation.clone());
    let threshold_proposal =
        ProposalState::new(*fixture.multisig_id.value(), 0, 1, 2, threshold.clone());
    let supply_id = AccountId::new([15; 32]);
    let mut result = vec![
        Case::gate(
            "initialize",
            vec![empty_account(fixture.multisig_id, true)],
            &QuorumInstruction::Initialize {
                threshold: 2,
                member_count: 3,
                member_root: fixture.constitution.member_root,
                tiers: fixture.constitution.tiers.clone(),
            },
            None,
        )?,
        Case::new(
            "create_token",
            programs::token(),
            vec![
                empty_account(fixture.definition_id, true),
                empty_account(supply_id, true),
            ],
            &TokenInstruction::NewFungibleDefinition {
                name: "QUORUM-BENCH".to_owned(),
                total_supply: 1_000,
            },
            None,
        )?,
        Case::new(
            "initialize_recipient",
            programs::token(),
            vec![
                token_definition_account(fixture)?,
                empty_account(fixture.recipient_id, true),
            ],
            &TokenInstruction::InitializeAccount,
            None,
        )?,
        Case::gate(
            "initialize_vault_gate",
            vec![
                constitution_account(fixture, true)?,
                token_definition_account(fixture)?,
                empty_account(fixture.vault_id, false),
            ],
            &QuorumInstruction::InitializeVault,
            None,
        )?,
        Case::new(
            "fund_vault",
            programs::token(),
            vec![
                token_account(supply_id, fixture.definition_id, 1_000, true)?,
                token_account(fixture.vault_id, fixture.definition_id, 0, false)?,
            ],
            &TokenInstruction::Transfer {
                amount_to_transfer: 750,
            },
            None,
        )?,
        propose_case(fixture, "propose_transfer", fixture.transfer.action.clone())?,
        propose_case(fixture, "propose_rotation", rotation)?,
        propose_case(fixture, "propose_threshold_change", threshold)?,
        Case::gate(
            "approve_one",
            vec![
                constitution_account(fixture, false)?,
                proposal_account(fixture, &fixture.transfer)?,
                empty_account(fixture.credential_id, true),
            ],
            &QuorumInstruction::Approve {
                proposal_id: 0,
                claim: fixture.approve_claim.clone(),
            },
            Some(fixture.approve_receipt.clone()),
        )?,
        execute_case(fixture, "execute_transfer", fixture.transfer.clone())?,
        execute_case(fixture, "execute_rotation", rotation_proposal)?,
        execute_case(fixture, "execute_threshold_change", threshold_proposal)?,
    ];
    result.sort_by_key(|case| case.operation);
    Ok(result)
}

fn main() -> Result<()> {
    if std::env::var("RISC0_DEV_MODE").ok().as_deref() != Some("1") {
        bail!("set RISC0_DEV_MODE=1; receipt mode does not change executor user-cycle counts");
    }
    let fixture = approval_fixture()?;
    println!("metric=LEZ Risc0 executor user_cycles");
    println!("lez_version=v0.2.2");
    println!("operation,user_cycles,segments,best_exec_ms");
    for case in cases(&fixture)? {
        let (cycles, segments, best_ms) = case.run()?;
        println!("{},{cycles},{segments},{best_ms:.3}", case.operation);
    }
    Ok(())
}
