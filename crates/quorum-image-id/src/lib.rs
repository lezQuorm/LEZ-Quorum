//! The pinned Risc0 image ID of the Quorum threshold guest.
//!
//! The on-chain gate verifies receipts against this ID, so it must match the
//! compiled guest exactly. Refresh it with `scripts/update-image-id.sh` after
//! any change to `guests/quorum-threshold/guest/`.

pub const THRESHOLD_IMAGE_ID: [u32; 8] = [
    2_579_077_875,
    769_874_733,
    529_682_050,
    4_062_924_364,
    2_705_577_364,
    2_680_433_381,
    735_259_384,
    241_280_473,
];
