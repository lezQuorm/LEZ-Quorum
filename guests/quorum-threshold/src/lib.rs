/// Exact threshold ELF accepted by the gate; pinned so its image ID is host-independent.
pub const THRESHOLD_ELF: &[u8] = include_bytes!("../artifacts/threshold.bin");
pub const THRESHOLD_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/threshold.bin");
pub const THRESHOLD_ID: [u32; 8] = [
    1_186_714_911,
    372_965_427,
    361_634_562,
    623_475_285,
    4_245_419_629,
    3_728_370_648,
    573_247_614,
    3_919_023_327,
];
