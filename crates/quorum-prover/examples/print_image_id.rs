//! Prints the compiled threshold guest image ID.

use quorum_threshold_methods::THRESHOLD_ELF;

fn main() {
    let image_id: [u32; 8] = risc0_zkvm::compute_image_id(THRESHOLD_ELF)
        .expect("compute_image_id should succeed for the compiled guest")
        .into();
    let words: Vec<String> = image_id.iter().copied().map(underscored).collect();
    println!("image_id rust: [{}]", words.join(", "));
    println!("image_id hex:  {image_id:?}");
}

/// Formats a u32 as a readable Rust literal.
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
