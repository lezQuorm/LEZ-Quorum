//! The pinned Risc0 image ID of the Quorum threshold guest.
//!
//! The on-chain gate verifies receipts against this ID, so it must match the
//! compiled guest exactly. Refresh it with `scripts/update-image-id.sh` after
//! any change to `guests/quorum-threshold/guest/`.

pub const THRESHOLD_IMAGE_ID: [u32; 8] = [
    114_484_643,
    2_738_439_775,
    93_721_807,
    2_809_967_440,
    468_656_058,
    4_246_638_024,
    2_892_828_720,
    3_001_232_771,
];
