include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
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
}
