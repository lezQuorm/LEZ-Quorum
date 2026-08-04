//! Prints the compiled Quorum threshold guest's image ID as a rust array.
//!
//! Fast and safe to run in any mode (`RISC0_DEV_MODE` does not affect image
//! ID computation — it is a pure function of the guest ELF). `update-image-id.sh`
//! consumes this to refresh `crates/quorum-image-id/src/lib.rs` after any change
//! to `guests/quorum-threshold/guest/`.
//!
//! ```bash
//! cargo run -p quorum-prover --example print_image_id
//! ```

use quorum_threshold_methods::THRESHOLD_ELF;

fn main() {
    let image_id: [u32; 8] = risc0_zkvm::compute_image_id(THRESHOLD_ELF)
        .expect("compute_image_id should succeed for the compiled guest")
        .into();
    // Underscored literals so the output can be pasted verbatim into
    // quorum-image-id (keeps `unreadable_literal` clippy happy).
    let words: Vec<String> = image_id.iter().copied().map(underscored).collect();
    println!("image_id rust: [{}]", words.join(", "));
    println!("image_id hex:  {image_id:?}");
}

/// Formats a u32 with underscore thousands separators (`2504793846` -> `2_504_793_846`).
fn underscored(value: u32) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push('_');
        }
        out.push(ch);
    }
    out
}
