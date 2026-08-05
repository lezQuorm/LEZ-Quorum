use std::path::PathBuf;

use spel_framework::idl::{
    IdlAccountType, IdlEnumVariant, IdlError, IdlField, IdlType, IdlTypeDef,
};

fn primitive(name: &str) -> IdlType {
    IdlType::Primitive(name.to_string())
}

fn defined(name: &str) -> IdlType {
    IdlType::Defined {
        defined: name.to_string(),
    }
}

fn array32() -> IdlType {
    IdlType::Array {
        array: (Box::new(primitive("u8")), 32),
    }
}

fn vec_of(type_: IdlType) -> IdlType {
    IdlType::Vec {
        vec: Box::new(type_),
    }
}

fn field(name: &str, type_: IdlType) -> IdlField {
    IdlField {
        name: name.to_string(),
        type_,
    }
}

fn struct_def(name: &str, fields: Vec<IdlField>) -> IdlTypeDef {
    IdlTypeDef {
        name: name.to_string(),
        kind: "struct".to_string(),
        fields,
        variants: Vec::new(),
    }
}

fn account_def(name: &str, fields: Vec<IdlField>) -> IdlAccountType {
    IdlAccountType {
        name: name.to_string(),
        type_: IdlTypeDef {
            name: String::new(),
            ..struct_def(name, fields)
        },
    }
}

fn complete_types() -> Vec<IdlTypeDef> {
    vec![
        IdlTypeDef {
            name: "ActionData".to_string(),
            kind: "enum".to_string(),
            fields: Vec::new(),
            variants: vec![
                IdlEnumVariant {
                    name: "Transfer".to_string(),
                    fields: vec![
                        field("recipient", array32()),
                        field("amount", primitive("u64")),
                        field("tier_id", primitive("u8")),
                        field("tier_max_amount", primitive("u64")),
                    ],
                },
                IdlEnumVariant {
                    name: "RotateMembers".to_string(),
                    fields: vec![
                        field("new_member_root", array32()),
                        field("new_member_count", primitive("u8")),
                    ],
                },
                IdlEnumVariant {
                    name: "ChangeThreshold".to_string(),
                    fields: vec![field("new_threshold", primitive("u8"))],
                },
            ],
        },
        struct_def(
            "OnChainThresholdJournal",
            vec![
                field("member_root", array32()),
                field("proposal_id", primitive("u64")),
                field("constitution_version", primitive("u32")),
                field("required_threshold", primitive("u8")),
                field("approval_count", primitive("u8")),
                field("nullifiers", vec_of(array32())),
                field("credential_commitments", vec_of(array32())),
                field("action", defined("ActionData")),
            ],
        ),
        IdlTypeDef {
            name: "ProposalStatus".to_string(),
            kind: "enum".to_string(),
            fields: Vec::new(),
            variants: vec![
                IdlEnumVariant {
                    name: "Active".to_string(),
                    fields: Vec::new(),
                },
                IdlEnumVariant {
                    name: "Executed".to_string(),
                    fields: Vec::new(),
                },
            ],
        },
        struct_def(
            "ThresholdClaim",
            vec![field("journal", defined("OnChainThresholdJournal"))],
        ),
        struct_def(
            "TierPolicy",
            vec![
                field("id", primitive("u8")),
                field("threshold", primitive("u8")),
                field("max_amount", primitive("u64")),
            ],
        ),
    ]
}

fn complete_accounts() -> Vec<IdlAccountType> {
    vec![
        account_def(
            "ConstitutionState",
            vec![
                field("multisig_id", array32()),
                field("version", primitive("u32")),
                field("threshold", primitive("u8")),
                field("member_count", primitive("u8")),
                field("member_root", array32()),
                field("tiers", vec_of(defined("TierPolicy"))),
                field("proposal_counter", primitive("u64")),
            ],
        ),
        account_def(
            "ProposalState",
            vec![
                field("multisig_id", array32()),
                field("id", primitive("u64")),
                field("constitution_version", primitive("u32")),
                field("threshold", primitive("u8")),
                field("action", defined("ActionData")),
                field("nullifiers", vec_of(array32())),
                field("status", defined("ProposalStatus")),
            ],
        ),
    ]
}

fn complete_errors() -> Vec<IdlError> {
    [
        (4001, "InvalidConstitution", "constitution is malformed"),
        (4002, "TierNotFound", "spending tier not found"),
        (4003, "DuplicateNullifier", "duplicate nullifier"),
        (4004, "ProposalNotActive", "proposal is not active"),
        (4005, "JournalMismatch", "journal does not match proposal"),
        (4006, "ThresholdMismatch", "proof threshold mismatch"),
        (4007, "NoopRotation", "rotation keeps the same root"),
        (
            4008,
            "RotationWouldBreakThreshold",
            "rotation breaks threshold",
        ),
        (4009, "InvalidThresholdChange", "invalid threshold change"),
        (4010, "StaleConstitution", "proof uses a stale constitution"),
        (4011, "TierCapMismatch", "journal tier cap mismatch"),
        (4012, "InvalidVault", "vault is not the treasury PDA"),
        (
            4013,
            "ProposalBindingMismatch",
            "proposal belongs to another multisig",
        ),
        (4014, "StaleProposal", "proposal uses a stale constitution"),
        (
            4015,
            "InvalidRecipient",
            "recipient differs from approved action",
        ),
        (
            4016,
            "ProposalIdMismatch",
            "instruction and account proposal ids differ",
        ),
        (
            4017,
            "CredentialMismatch",
            "credential accounts do not match proof",
        ),
    ]
    .into_iter()
    .map(|(code, name, message)| IdlError {
        code,
        name: name.to_string(),
        msg: Some(message.to_string()),
    })
    .collect()
}

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = manifest.join("guest/src/bin/quorum_gate.rs");
    let mut idl = spel_framework_core::idl_gen::generate_idl_from_file(&source)
        .expect("quorum gate source must produce an IDL");
    idl.accounts = complete_accounts();
    idl.types = complete_types();
    idl.errors = complete_errors();

    let output = manifest.join("idl/quorum_gate.idl.json");
    let json = idl.to_json_pretty().expect("IDL must serialize");
    std::fs::write(&output, format!("{json}\n")).expect("IDL file must be writable");
    println!("wrote {}", output.display());
}
