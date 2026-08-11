/// Exact gate ELF deployed to LEZ testnet; pinned so its program ID is host-independent.
pub const QUORUM_GATE_ELF: &[u8] = include_bytes!("../artifacts/quorum_gate.bin");
pub const QUORUM_GATE_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/quorum_gate.bin");
pub const QUORUM_GATE_ID: [u32; 8] = [
    320_098_040,
    1_020_072_060,
    2_381_930_866,
    4_243_020_391,
    4_177_030_334,
    802_000_452,
    1_921_768_834,
    3_969_437_236,
];

#[cfg(test)]
mod tests {
    use quorum_gate_core::{
        ActionData, OnChainThresholdJournal, QuorumInstruction, ThresholdClaim, TierPolicy,
        TokenInitializeInstruction,
    };

    /// Byte-level guard for `quorum_gate_core::TokenTransferInstruction`.
    ///
    /// The on-chain `execute` handler builds the transfer `ChainedCall`
    /// instruction data from this serde mirror. `ChainedCall::new` serializes
    /// with `risc0_zkvm::serde::to_vec` and the token program decodes with the
    /// same deserializer, so the mirror must produce byte-identical words to
    /// `token_core::Instruction::Transfer`. This test compares them directly
    /// against the real LEZ type (dev-dependency on the same git repo the
    /// guest already pins for `nssa_core`).
    #[test]
    fn transfer_instruction_mirror_matches_token_core_bytes() {
        let amount = 500_u128;
        let mirror = quorum_gate_core::TokenTransferInstruction::Transfer {
            amount_to_transfer: amount,
        };
        let real = token_core::Instruction::Transfer {
            amount_to_transfer: amount,
        };
        assert_eq!(
            risc0_zkvm::serde::to_vec(&mirror).unwrap(),
            risc0_zkvm::serde::to_vec(&real).unwrap(),
            "TokenTransferInstruction serde mirror drifted from token_core::Instruction"
        );
    }

    #[test]
    fn initialize_instruction_mirror_matches_token_core_bytes() {
        assert_eq!(
            risc0_zkvm::serde::to_vec(&TokenInitializeInstruction::InitializeAccount).unwrap(),
            risc0_zkvm::serde::to_vec(&token_core::Instruction::InitializeAccount).unwrap(),
            "TokenInitializeInstruction serde mirror drifted from token_core::Instruction"
        );
    }

    #[test]
    fn complete_idl_generates_client_and_covers_instruction_codec() {
        let idl_json = include_str!("../idl/quorum_gate.idl.json");
        let idl: spel_framework::idl::SpelIdl =
            spel_framework::serde_json::from_str(idl_json).expect("valid gate IDL");
        let approve = idl
            .instructions
            .iter()
            .find(|instruction| instruction.name == "approve")
            .expect("approve instruction");
        let credentials = approve
            .accounts
            .iter()
            .find(|account| account.name == "credentials")
            .expect("credential rest accounts");
        assert!(credentials.rest && credentials.signer && credentials.writable);

        for required in [
            "ActionData",
            "OnChainThresholdJournal",
            "ProposalStatus",
            "ThresholdClaim",
            "TierPolicy",
        ] {
            assert!(idl.types.iter().any(|type_| type_.name == required));
        }
        for required in ["ConstitutionState", "ProposalState"] {
            assert!(idl.accounts.iter().any(|account| account.name == required));
        }

        let generated =
            spel_client_gen::generate_from_idl_json(idl_json).expect("client generation");
        assert!(generated
            .client_code
            .contains("pub credentials: Vec<AccountId>"));
        assert!(generated.ffi_code.contains("credential_commitments"));

        let journal = OnChainThresholdJournal {
            member_root: [1_u8; 32],
            proposal_id: 3,
            constitution_version: 2,
            required_threshold: 2,
            approval_count: 2,
            nullifiers: vec![[2_u8; 32], [3_u8; 32]],
            credential_commitments: vec![[4_u8; 32], [5_u8; 32]],
            action: ActionData::ChangeThreshold { new_threshold: 2 },
        };
        let instructions = [
            QuorumInstruction::Initialize {
                threshold: 2,
                member_count: 3,
                member_root: [6_u8; 32],
                tiers: vec![TierPolicy {
                    id: 1,
                    threshold: 2,
                    max_amount: 1_000,
                }],
            },
            QuorumInstruction::Propose {
                action: ActionData::RotateMembers {
                    new_member_root: [7_u8; 32],
                    new_member_count: 4,
                },
            },
            QuorumInstruction::Approve {
                proposal_id: 3,
                claim: ThresholdClaim { journal },
            },
            QuorumInstruction::Execute { proposal_id: 3 },
            QuorumInstruction::InitializeVault,
        ];
        for instruction in instructions {
            let words = risc0_zkvm::serde::to_vec(&instruction).expect("instruction encoding");
            let decoded: QuorumInstruction =
                risc0_zkvm::serde::from_slice(&words).expect("instruction decoding");
            assert_eq!(decoded, instruction);
        }
    }
}
