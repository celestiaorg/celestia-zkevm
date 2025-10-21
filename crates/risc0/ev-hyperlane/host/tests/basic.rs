//! Basic integration test for RISC0 EV-Hyperlane circuit

use ev_hyperlane_host::{EV_HYPERLANE_ELF, EV_HYPERLANE_ID, EV_HYPERLANE_IMAGE_ID};

#[test]
fn test_image_id_is_valid() {
    // Verify the ImageID is not all zeros
    assert_ne!(EV_HYPERLANE_ID, [0u32; 8], "ImageID should not be all zeros");

    // Verify the ImageID bytes are correctly derived
    let mut expected_bytes = [0u8; 32];
    for (i, &word) in EV_HYPERLANE_ID.iter().enumerate() {
        let bytes = word.to_le_bytes();
        expected_bytes[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
    }

    assert_eq!(EV_HYPERLANE_IMAGE_ID, expected_bytes, "ImageID bytes should match");
}

#[test]
fn test_elf_is_not_empty() {
    // Verify the ELF binary is embedded and not empty
    assert!(!EV_HYPERLANE_ELF.is_empty(), "ELF binary should not be empty");

    // Verify it's a reasonable size (guest binaries are typically > 100KB)
    assert!(EV_HYPERLANE_ELF.len() > 100_000, "ELF binary should be at least 100KB");
}

#[test]
fn test_image_id_consistency() {
    // Print the ImageID for debugging
    println!("RISC0 EV-Hyperlane ImageID (u32): {:?}", EV_HYPERLANE_ID);
    println!("RISC0 EV-Hyperlane ImageID (bytes): {:?}", &EV_HYPERLANE_IMAGE_ID[..8]);

    // The ImageID should be deterministic
    let expected_id = [2744069191, 1830777240, 326310898, 4215601095, 1007629970, 1316900118, 4060872798, 1737830579];

    if EV_HYPERLANE_ID != expected_id {
        eprintln!("WARNING: ImageID has changed!");
        eprintln!("Old: {:?}", expected_id);
        eprintln!("New: {:?}", EV_HYPERLANE_ID);
        eprintln!("Update RISC0_CONSTANTS.rs if this was intentional");
    }
}
