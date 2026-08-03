//! The pinned Risc0 image ID of the Quorum threshold guest.
//!
//! The on-chain gate verifies receipts against this ID, so it must match the
//! compiled guest exactly. Refresh it with `scripts/update-image-id.sh` after
//! any change to `guests/quorum-threshold/guest/`.

pub const THRESHOLD_IMAGE_ID: [u32; 8] = [
    2_504_793_846,
    1_302_641_585,
    509_407_582,
    452_779_787,
    1_019_694_882,
    662_766_674,
    1_532_127_949,
    2_008_668_271,
];
