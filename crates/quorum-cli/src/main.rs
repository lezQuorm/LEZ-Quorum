//! # Quorum CLI
//!
//! Offline-first multisig tooling. Everything runs locally: create the member
//! set + constitution, propose, generate **client-side approval proofs**,
//! aggregate, and apply. On-chain submission (the claim artifacts) is
//! documented in `docs/DEPLOYMENT.md` and the demo runbook.
//!
//! ```bash
//! quorum create --threshold 2 --members 3
//! quorum propose --action transfer --recipient <hex> --amount 500 --tier 1
//! quorum approve --member 0 --proposal 0        # real proof (RISC0_DEV_MODE=0)
//! quorum approve --member 1 --proposal 0
//! quorum execute --proposal 0
//! quorum info
//! # Aggregated single-proof mode (B3): M approvals in ONE receipt
//! quorum approve-all --proposal 0 --members 0,1
//! ```

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use quorum_circuit::ActionData;
use quorum_gate_core::TierPolicy;
use quorum_sdk::{Member, MemberSet, Multisig};
use serde::{Deserialize, Serialize};

const STATE_FILE: &str = "quorum.json";
const MEMBER_FILE_PREFIX: &str = "member-";
const CLAIMS_DIR: &str = "claims";

/// The local state file: multisig mirror + member commitments + secrets.
/// Permissions are forced to 0600 — never commit this file.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct QuorumFile {
    multisig: Multisig,
    commitments: Vec<[u8; 32]>,
}

#[derive(Parser)]
#[command(
    name = "quorum",
    version,
    about = "Private M-of-N multisig for LEZ (LP-0002)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a multisig: writes quorum.json + member-<i>.json (secrets, 0600).
    Create {
        /// Required approvals M.
        #[arg(long)]
        threshold: u8,
        /// Number of members N.
        #[arg(long)]
        members: usize,
        /// Optional tiers as JSON: `[{"id":1,"threshold":2,"max_amount":1000}]`
        #[arg(long)]
        tiers: Option<String>,
    },
    /// Open a proposal.
    Propose {
        /// transfer | rotate | threshold
        #[arg(long)]
        action: String,
        /// Recipient hex (transfer).
        #[arg(long)]
        recipient: Option<String>,
        /// Amount (transfer).
        #[arg(long)]
        amount: Option<u64>,
        /// Tier id (transfer).
        #[arg(long)]
        tier: Option<u8>,
        /// New member root hex (rotate).
        #[arg(long)]
        new_member_root: Option<String>,
        /// New member count (rotate).
        #[arg(long)]
        new_member_count: Option<u8>,
        /// New default threshold (threshold change).
        #[arg(long)]
        new_threshold: Option<u8>,
    },
    /// Generate a member's approval proof (client-side) and record it.
    Approve {
        /// Member index (0-based).
        #[arg(long)]
        member: usize,
        /// Proposal id.
        #[arg(long)]
        proposal: u64,
    },
    /// Generate ONE aggregated threshold proof for several members (B3:
    /// M distinct approvals in a single receipt, single on-chain claim).
    ApproveAll {
        /// Proposal id.
        #[arg(long)]
        proposal: u64,
        /// Comma-separated member indexes, e.g. `0,1`.
        #[arg(long)]
        members: String,
    },
    /// Execute a proposal once the threshold is met.
    Execute {
        /// Proposal id.
        #[arg(long)]
        proposal: u64,
    },
    /// Reject a proposal.
    Reject {
        /// Proposal id.
        #[arg(long)]
        proposal: u64,
    },
    /// Print the current state.
    Info,
    /// Print the member root for a freshly generated member set (rotation helper).
    NewRoot {
        /// Number of members in the new set.
        #[arg(long)]
        members: usize,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Create {
            threshold,
            members,
            tiers,
        } => create(threshold, members, tiers.as_deref()),
        Command::Propose {
            action,
            recipient,
            amount,
            tier,
            new_member_root,
            new_member_count,
            new_threshold,
        } => propose(
            action.as_str(),
            recipient.as_deref(),
            amount,
            tier,
            new_member_root.as_deref(),
            new_member_count,
            new_threshold,
        ),
        Command::Approve { member, proposal } => approve(member, proposal),
        Command::ApproveAll { proposal, members } => approve_all(proposal, &members),
        Command::Execute { proposal } => execute(proposal),
        Command::Reject { proposal } => reject(proposal),
        Command::Info => info(),
        Command::NewRoot { members } => {
            if members == 0 || members > 10 {
                return Err("member count must be 1..=10".into());
            }
            let set = MemberSet::generate(members);
            println!("{}", hex(&set.root));
            Ok(())
        }
    }
}

fn create(threshold: u8, members: usize, tiers: Option<&str>) -> Result<(), String> {
    if members == 0 || members > 10 {
        return Err("member count must be 1..=10".into());
    }
    let set = MemberSet::generate(members);
    let tiers = parse_tiers(tiers)?;
    let multisig = Multisig::create(threshold, &set, tiers).map_err(|e| e.to_string())?;

    let state = QuorumFile {
        commitments: set.members.iter().map(Member::commitment).collect(),
        multisig,
    };
    write_state(&state)?;
    for member in &set.members {
        write_secret(member.index, &member.secret)?;
    }
    std::fs::create_dir_all(CLAIMS_DIR).map_err(|e| e.to_string())?;

    println!("multisig created:");
    println!("  threshold:   {threshold}");
    println!("  members:     {members}");
    println!(
        "  member_root: {}",
        hex(&state.multisig.constitution.member_root)
    );
    Ok(())
}

fn parse_tiers(tiers: Option<&str>) -> Result<Vec<TierPolicy>, String> {
    match tiers {
        None => Ok(Vec::new()),
        Some(json) => serde_json::from_str(json).map_err(|e| format!("invalid tiers JSON: {e}")),
    }
}

fn propose(
    action: &str,
    recipient: Option<&str>,
    amount: Option<u64>,
    tier: Option<u8>,
    new_member_root: Option<&str>,
    new_member_count: Option<u8>,
    new_threshold: Option<u8>,
) -> Result<(), String> {
    let mut state = load_state()?;
    let action = match action {
        "transfer" => ActionData::Transfer {
            recipient: parse_hex32(recipient.ok_or("--recipient required")?)?,
            amount: amount.ok_or("--amount required")?,
            tier_id: tier.ok_or("--tier required")?,
            tier_max_amount: state
                .multisig
                .constitution
                .tiers
                .iter()
                .find(|t| t.id == tier.unwrap_or(0))
                .ok_or("unknown tier")?
                .max_amount,
        },
        "rotate" => ActionData::RotateMembers {
            new_member_root: parse_hex32(new_member_root.ok_or("--new-member-root required")?)?,
            new_member_count: new_member_count.ok_or("--new-member-count required")?,
        },
        "threshold" => ActionData::ChangeThreshold {
            new_threshold: new_threshold.ok_or("--new-threshold required")?,
        },
        other => {
            return Err(format!(
                "unknown action '{other}'; use transfer|rotate|threshold"
            ))
        }
    };
    let id = state.multisig.propose(action).map_err(|e| e.to_string())?;
    write_state(&state)?;
    println!("proposal {id} opened");
    Ok(())
}

fn approve(member_index: usize, proposal_id: u64) -> Result<(), String> {
    let mut state = load_state()?;
    let secret = load_secret(member_index)?;
    let member = Member {
        index: member_index,
        secret,
    };
    let proof = state
        .multisig
        .approve(proposal_id, &state.commitments, &member)
        .map_err(|e| e.to_string())?;
    write_state(&state)?;
    let claim_path = Path::new(CLAIMS_DIR).join(format!("claim-{proposal_id}-{member_index}.json"));
    let claim = serde_json::to_string_pretty(&proof).map_err(|e| e.to_string())?;
    write_private(&claim_path, claim.as_bytes())?;
    println!(
        "approval recorded for proposal {proposal_id} (nullifier {})",
        hex(&proof.journal.nullifiers[0])
    );
    println!("claim written: {}", claim_path.display());
    Ok(())
}

fn approve_all(proposal_id: u64, members_csv: &str) -> Result<(), String> {
    let mut state = load_state()?;
    let indexes: Vec<usize> = members_csv
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<usize>()
                .map_err(|e| format!("invalid member index '{s}': {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if indexes.is_empty() {
        return Err("--members must list at least one index".into());
    }
    let members: Vec<Member> = indexes
        .iter()
        .map(|&index| {
            let secret = load_secret(index)?;
            Ok(Member { index, secret })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let member_refs: Vec<&Member> = members.iter().collect();

    let proof = state
        .multisig
        .approve_many(proposal_id, &state.commitments, &member_refs)
        .map_err(|e| e.to_string())?;
    write_state(&state)?;

    let claim_path = Path::new(CLAIMS_DIR).join(format!("claim-{proposal_id}-aggregated.json"));
    let claim = serde_json::to_string_pretty(&proof).map_err(|e| e.to_string())?;
    write_private(&claim_path, claim.as_bytes())?;

    let nullifiers: Vec<String> = proof.journal.nullifiers.iter().map(|n| hex(n)).collect();
    println!(
        "aggregated approval recorded for proposal {proposal_id} ({} members, one proof)",
        proof.journal.approval_count
    );
    println!("nullifiers: {nullifiers:?}");
    println!("claim written: {}", claim_path.display());
    Ok(())
}

fn execute(proposal_id: u64) -> Result<(), String> {
    let mut state = load_state()?;
    state
        .multisig
        .execute(proposal_id)
        .map_err(|e| e.to_string())?;
    write_state(&state)?;
    println!("proposal {proposal_id} executed");
    Ok(())
}

fn reject(proposal_id: u64) -> Result<(), String> {
    let mut state = load_state()?;
    state
        .multisig
        .reject(proposal_id)
        .map_err(|e| e.to_string())?;
    write_state(&state)?;
    println!("proposal {proposal_id} rejected");
    Ok(())
}

fn info() -> Result<(), String> {
    let state = load_state()?;
    let c = &state.multisig.constitution;
    println!("constitution v{}", c.version);
    println!("  threshold:   {}", c.threshold);
    println!("  members:     {}", c.member_count);
    println!("  member_root: {}", hex(&c.member_root));
    println!(
        "  tiers:       {}",
        serde_json::to_string(&c.tiers).unwrap_or_default()
    );
    for p in &state.multisig.proposals {
        println!(
            "  proposal {}: status={:?} approvals={}/{} action={:?}",
            p.id,
            p.status,
            p.nullifiers.len(),
            p.threshold,
            p.action
        );
    }
    Ok(())
}

fn parse_hex32(value: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(value).map_err(|e| format!("invalid hex: {e}"))?;
    bytes
        .try_into()
        .map_err(|_| "expected exactly 32 bytes (64 hex chars)".into())
}

fn load_state() -> Result<QuorumFile, String> {
    let bytes = std::fs::read(STATE_FILE).map_err(|e| format!("cannot read {STATE_FILE}: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("cannot parse {STATE_FILE}: {e}"))
}

fn write_state(state: &QuorumFile) -> Result<(), String> {
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    write_private(Path::new(STATE_FILE), json.as_bytes())
}

fn write_secret(index: usize, secret: &[u8; 32]) -> Result<(), String> {
    let path = PathBuf::from(format!("{MEMBER_FILE_PREFIX}{index}.json"));
    write_private(
        &path,
        &serde_json::to_vec(&secret).map_err(|e| e.to_string())?,
    )
}

fn load_secret(index: usize) -> Result<[u8; 32], String> {
    let path = PathBuf::from(format!("{MEMBER_FILE_PREFIX}{index}.json"));
    let bytes = std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("cannot parse {}: {e}", path.display()))
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| e.to_string())
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .fold(String::new(), |mut s, h| {
            s.push_str(&h);
            s
        })
}
