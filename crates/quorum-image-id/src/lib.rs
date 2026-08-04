//! The pinned Risc0 image ID of the Quorum threshold guest.
//!
//! The on-chain gate verifies receipts against this ID, so it must match the
//! compiled guest exactly. Refresh it with `scripts/update-image-id.sh` after
//! any change to `guests/quorum-threshold/guest/`.

pub const THRESHOLD_IMAGE_ID: [u32; 8] = [
    3_200_284_588,
    1_852_504_360,
    2_332_593_133,
    3_866_069_938,
    4_186_485_082,
    2_581_798_040,
    3_100_454_683,
    3_649_897_487,
];
