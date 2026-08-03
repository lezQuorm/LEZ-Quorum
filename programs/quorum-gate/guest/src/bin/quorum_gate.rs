#![no_main]

use quorum_gate_core::{
    apply_action, apply_approved_claim, check_claim, decode_constitution, decode_proposal,
    encode_constitution, encode_proposal, ConstitutionState, ProposalState, ProposalStatus,
    QuorumInstruction, ThresholdClaim, THRESHOLD_IMAGE_ID,
};
use nssa_core::{
    account::AccountWithMetadata,
    program::{AccountPostState, Claim},
};
use risc0_zkvm::{guest::env, serde::to_vec};
use spel_framework::prelude::*;

risc0_zkvm::guest::entry!(main);

#[lez_program(instruction = "quorum_gate_core::QuorumInstruction")]
mod quorum_gate {
    #[allow(unused_imports)]
    use super::*;

    #[instruction]
    pub fn initialize(
        _ctx: ProgramContext,
        #[account(init, signer)] mut multisig: AccountWithMetadata,
        threshold: u8,
        member_count: u8,
        member_root: [u8; 32],
        tiers: Vec<quorum_gate_core::TierPolicy>,
    ) -> SpelResult {
        let state = ConstitutionState::new(threshold, member_count, member_root, tiers)
            .unwrap_or_else(|error| fail(error.code() as u16, &error.to_string()));
        multisig.account.data = encode_constitution(&state)
            .unwrap_or_else(|_| fail(2005, "cannot encode constitution state"))
            .try_into()
            .unwrap_or_else(|_| fail(2005, "constitution state is too large"));

        let mut output = SpelOutput::empty();
        output.post_states = vec![AccountPostState::new_claimed(
            multisig.account,
            Claim::Authorized,
        )];
        Ok(output)
    }

    #[instruction]
    pub fn propose(
        ctx: ProgramContext,
        #[account(mut, owner = self_program_id)] mut multisig: AccountWithMetadata,
        #[account(init, signer)] mut proposal: AccountWithMetadata,
        action: quorum_gate_core::ActionData,
    ) -> SpelResult {
        let mut constitution = decode_constitution(&multisig.account.data)
            .unwrap_or_else(|_| fail(2005, "cannot decode constitution state"));
        constitution.validate().unwrap_or_else(|error| fail(error.code() as u16, &error.to_string()));

        let threshold = constitution
            .required_threshold(&action)
            .unwrap_or_else(|error| fail(error.code() as u16, &error.to_string()));
        let id = constitution.proposal_counter;
        let state = ProposalState::new(id, constitution.version, threshold, action);
        proposal.account.data = encode_proposal(&state)
            .unwrap_or_else(|_| fail(2005, "cannot encode proposal state"))
            .try_into()
            .unwrap_or_else(|_| fail(2005, "proposal state is too large"));

        constitution.proposal_counter = constitution.proposal_counter.saturating_add(1);
        multisig.account.data = encode_constitution(&constitution)
            .unwrap_or_else(|_| fail(2005, "cannot encode constitution state"))
            .try_into()
            .unwrap_or_else(|_| fail(2005, "constitution state is too large"));

        let mut output = SpelOutput::empty();
        output.post_states = vec![
            AccountPostState::new(multisig.account),
            AccountPostState::new_claimed(proposal.account, Claim::Authorized),
        ];
        let _ = ctx;
        Ok(output)
    }

    #[instruction]
    pub fn approve(
        _ctx: ProgramContext,
        #[account(mut, owner = self_program_id)] multisig: AccountWithMetadata,
        #[account(mut, owner = self_program_id)] mut proposal: AccountWithMetadata,
        proposal_id: u64,
        claim: ThresholdClaim,
    ) -> SpelResult {
        let _ = proposal_id; // the account is authoritative; id kept for macro dispatch
        let constitution = decode_constitution(&multisig.account.data)
            .unwrap_or_else(|_| fail(2005, "cannot decode constitution state"));
        let mut state = decode_proposal(&proposal.account.data)
            .unwrap_or_else(|_| fail(2005, "cannot decode proposal state"));

        // Bind the claim to this program and proposal, then verify the client
        // receipt ON-CHAIN against the pinned threshold-guest image ID.
        let journal_words = to_vec(&claim.journal)
            .unwrap_or_else(|_| fail(1011, "cannot encode threshold journal"));
        env::verify(THRESHOLD_IMAGE_ID, &journal_words)
            .expect("Risc0 receipt verification is infallible inside the guest");

        let check = check_claim(&constitution, &state, &claim.journal)
            .unwrap_or_else(|error| fail(error.code() as u16, &error.to_string()));
        apply_approved_claim(&mut state, &check)
            .unwrap_or_else(|error| fail(error.code() as u16, &error.to_string()));

        proposal.account.data = encode_proposal(&state)
            .unwrap_or_else(|_| fail(2005, "cannot encode proposal state"))
            .try_into()
            .unwrap_or_else(|_| fail(2005, "proposal state is too large"));

        let mut output = SpelOutput::empty();
        output.post_states = vec![AccountPostState::new(proposal.account)];
        Ok(output)
    }

    #[instruction]
    pub fn execute(
        _ctx: ProgramContext,
        #[account(mut, owner = self_program_id)] mut multisig: AccountWithMetadata,
        #[account(mut, owner = self_program_id)] mut proposal: AccountWithMetadata,
        proposal_id: u64,
    ) -> SpelResult {
        let _ = proposal_id;
        let mut constitution = decode_constitution(&multisig.account.data)
            .unwrap_or_else(|_| fail(2005, "cannot decode constitution state"));
        let mut state = decode_proposal(&proposal.account.data)
            .unwrap_or_else(|_| fail(2005, "cannot decode proposal state"));

        if !state.threshold_met() {
            fail(4004, "proposal threshold not met");
        }
        apply_action(&mut constitution, &state)
            .unwrap_or_else(|error| fail(error.code() as u16, &error.to_string()));
        state.status = ProposalStatus::Executed;

        multisig.account.data = encode_constitution(&constitution)
            .unwrap_or_else(|_| fail(2005, "cannot encode constitution state"))
            .try_into()
            .unwrap_or_else(|_| fail(2005, "constitution state is too large"));
        proposal.account.data = encode_proposal(&state)
            .unwrap_or_else(|_| fail(2005, "cannot encode proposal state"))
            .try_into()
            .unwrap_or_else(|_| fail(2005, "proposal state is too large"));

        let mut output = SpelOutput::empty();
        output.post_states = vec![
            AccountPostState::new(multisig.account),
            AccountPostState::new(proposal.account),
        ];
        Ok(output)
    }

    #[instruction]
    pub fn reject(
        _ctx: ProgramContext,
        #[account(mut, owner = self_program_id)] mut proposal: AccountWithMetadata,
        proposal_id: u64,
    ) -> SpelResult {
        let _ = proposal_id;
        let mut state = decode_proposal(&proposal.account.data)
            .unwrap_or_else(|_| fail(2005, "cannot decode proposal state"));
        if state.status != ProposalStatus::Active {
            fail(4004, "proposal is not active");
        }
        state.status = ProposalStatus::Rejected;
        proposal.account.data = encode_proposal(&state)
            .unwrap_or_else(|_| fail(2005, "cannot encode proposal state"))
            .try_into()
            .unwrap_or_else(|_| fail(2005, "proposal state is too large"));

        let mut output = SpelOutput::empty();
        output.post_states = vec![AccountPostState::new(proposal.account)];
        Ok(output)
    }
}

fn fail(code: u16, message: &str) -> ! {
    panic!("Quorum gate error {code}: {message}")
}
