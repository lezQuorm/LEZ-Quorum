//! The pinned Risc0 image ID of the Quorum threshold guest.
//!
//! The on-chain gate verifies receipts against this ID, so it must match the
//! compiled guest exactly. Refresh it with `scripts/update-image-id.sh` after
//! any change to `guests/quorum-threshold/guest/`.

pub const THRESHOLD_IMAGE_ID: [u32; 8] = [
    1_186_714_911,
    372_965_427,
    361_634_562,
    623_475_285,
    4_245_419_629,
    3_728_370_648,
    573_247_614,
    3_919_023_327,
];
