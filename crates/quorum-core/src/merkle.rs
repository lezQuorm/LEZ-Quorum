//! Binary Merkle tree over member commitments.
//!
//! Hashing matches LEZ semantics exactly (see `lez-compat`):
//! `leaf = SHA256(member_commitment)`, `node = SHA256(left || right)`.
//! The tree is padded to the next power of two with the leaf hash of the zero
//! commitment so the root is deterministic for any member count.
//!
//! Leaves are sorted (canonical order), so the root is order-independent: two
//! Conclave instances with the same member set always derive the same root.

use sha2::{Digest, Sha256};

use crate::Commitment;

/// Leaf hash of the all-zero commitment — used as the padding leaf.
#[must_use]
pub fn zero_leaf() -> [u8; 32] {
    Sha256::digest([0_u8; 32]).into()
}

/// Leaf hash of a member commitment (LEZ `commitment.leaf_hash()`).
#[must_use]
pub fn leaf_hash(commitment: &Commitment) -> [u8; 32] {
    Sha256::digest(commitment).into()
}

/// Internal node hash: `SHA256(left || right)`.
#[must_use]
pub fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut pair = [0_u8; 64];
    pair[..32].copy_from_slice(left);
    pair[32..].copy_from_slice(right);
    Sha256::digest(pair).into()
}

/// A Merkle membership proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MembershipProof {
    /// Position of the leaf.
    pub leaf_index: usize,
    /// Sibling hashes from leaf to root.
    pub siblings: Vec<[u8; 32]>,
}

impl MembershipProof {
    /// Recomputes the root for a leaf.
    #[must_use]
    pub fn compute_root(&self, leaf: &[u8; 32]) -> [u8; 32] {
        let mut result = *leaf;
        let mut level_index = self.leaf_index;
        for sibling in &self.siblings {
            result = if level_index & 1 == 0 {
                node_hash(&result, sibling)
            } else {
                node_hash(sibling, &result)
            };
            level_index >>= 1;
        }
        result
    }

    /// Verifies a leaf against an expected root.
    #[must_use]
    pub fn verifies(&self, leaf: &[u8; 32], expected_root: &[u8; 32]) -> bool {
        self.compute_root(leaf) == *expected_root
    }
}

/// A Merkle tree over member commitments (canonically ordered).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberTree {
    levels: Vec<Vec<[u8; 32]>>,
}

impl MemberTree {
    /// Builds the tree from member commitments.
    ///
    /// # Panics
    /// Never in practice: `new` guarantees at least one tree level exists.
    #[must_use]
    pub fn new(commitments: &[Commitment]) -> Self {
        let mut leaves: Vec<[u8; 32]> = commitments.iter().map(leaf_hash).collect();
        leaves.sort_unstable();
        let size = leaves.len().max(1).next_power_of_two();
        leaves.resize(size, zero_leaf());

        let mut levels = vec![leaves];
        while levels.last().expect("at least one level").len() > 1 {
            let current = levels.last().expect("level exists");
            let next: Vec<[u8; 32]> = current
                .chunks_exact(2)
                .map(|pair| node_hash(&pair[0], &pair[1]))
                .collect();
            levels.push(next);
        }
        Self { levels }
    }

    /// The root commitment of the member set.
    ///
    /// # Panics
    /// Never in practice: `MemberTree::new` guarantees a root level exists.
    #[must_use]
    pub fn root(&self) -> [u8; 32] {
        self.levels.last().expect("root level exists")[0]
    }

    /// Number of actual (non-padding) leaves.
    #[must_use]
    pub fn len(&self) -> usize {
        // The first level includes padding; recover the real count by stripping
        // padding leaves from the end.
        let all = &self.levels[0];
        all.iter()
            .rev()
            .position(|l| *l != zero_leaf())
            .map_or(0, |p| all.len() - p)
    }

    /// Whether the tree has no members.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Computes the membership proof for the member at the *canonical sorted*
    /// index. Returns `None` for padding positions.
    #[must_use]
    pub fn proof(&self, canonical_index: usize) -> Option<MembershipProof> {
        let leaves = &self.levels[0];
        if canonical_index >= leaves.len() || leaves[canonical_index] == zero_leaf() {
            return None;
        }
        let mut idx = canonical_index;
        let mut siblings = Vec::with_capacity(self.levels.len() - 1);
        for level in &self.levels[..self.levels.len() - 1] {
            let sibling = if idx & 1 == 0 { idx + 1 } else { idx - 1 };
            siblings.push(level[sibling]);
            idx >>= 1;
        }
        Some(MembershipProof {
            leaf_index: canonical_index,
            siblings,
        })
    }

    /// Proof for a specific member commitment (finds its canonical index).
    #[must_use]
    pub fn proof_for(&self, commitment: &Commitment) -> Option<MembershipProof> {
        let leaf = leaf_hash(commitment);
        self.levels[0]
            .iter()
            .position(|l| *l == leaf)
            .and_then(|i| self.proof(i))
    }
}

/// Computes the member root for a set of member secrets (test/demo helper).
#[must_use]
pub fn member_root(secrets: &[[u8; 32]]) -> Commitment {
    let commitments: Vec<Commitment> = secrets
        .iter()
        .map(crate::nullifier::member_commitment)
        .collect();
    MemberTree::new(&commitments).root()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nullifier::member_commitment;

    #[allow(clippy::cast_possible_truncation)] // test helper: n < 256 always
    fn secrets(n: usize) -> Vec<[u8; 32]> {
        (0..n).map(|i| [i as u8; 32]).collect()
    }

    #[test]
    fn single_member_root_is_its_leaf() {
        let s = [7u8; 32];
        let c = member_commitment(&s);
        let tree = MemberTree::new(&[c]);
        assert_eq!(tree.root(), leaf_hash(&c));
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn proof_verifies_for_every_member() {
        let commitments: Vec<Commitment> = secrets(5).iter().map(member_commitment).collect();
        let tree = MemberTree::new(&commitments);
        for c in &commitments {
            let p = tree.proof_for(c).expect("member has proof");
            assert!(p.verifies(&leaf_hash(c), &tree.root()));
        }
    }

    #[test]
    fn tampered_sibling_fails() {
        let commitments: Vec<Commitment> = secrets(3).iter().map(member_commitment).collect();
        let tree = MemberTree::new(&commitments);
        let c = &commitments[0];
        let mut p = tree.proof_for(c).unwrap();
        p.siblings[0][0] ^= 1;
        assert!(!p.verifies(&leaf_hash(c), &tree.root()));
    }

    #[test]
    fn root_is_order_independent() {
        let a = member_commitment(&[1u8; 32]);
        let b = member_commitment(&[2u8; 32]);
        let c = member_commitment(&[3u8; 32]);
        assert_eq!(
            MemberTree::new(&[a, b, c]).root(),
            MemberTree::new(&[c, a, b]).root()
        );
    }

    #[test]
    fn rotation_changes_root_and_retires_old_member() {
        let old = member_commitment(&[1u8; 32]);
        let keep = member_commitment(&[2u8; 32]);
        let newcomer = member_commitment(&[4u8; 32]);

        let old_tree = MemberTree::new(&[old, keep]);
        let new_tree = MemberTree::new(&[keep, newcomer]);

        assert_ne!(old_tree.root(), new_tree.root());
        // The removed member's commitment is not in the new tree.
        assert!(new_tree.proof_for(&old).is_none());
        // The keeper still has a valid proof in both.
        let p = new_tree.proof_for(&keep).unwrap();
        assert!(p.verifies(&leaf_hash(&keep), &new_tree.root()));
        let _ = old_tree;
    }
}
