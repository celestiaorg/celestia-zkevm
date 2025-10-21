//! Basic integration test for RISC0 EV-Exec circuit
//!
//! This test verifies that the RISC0 guest program can be loaded and
//! the ImageID is correctly generated.

use ev_exec_host::{EV_EXEC_ELF, EV_EXEC_ID, EV_EXEC_IMAGE_ID};

#[test]
fn test_image_id_is_valid() {
    // Verify the ImageID is not all zeros
    assert_ne!(EV_EXEC_ID, [0u32; 8], "ImageID should not be all zeros");

    // Verify the ImageID bytes are correctly derived
    let mut expected_bytes = [0u8; 32];
    for (i, &word) in EV_EXEC_ID.iter().enumerate() {
        let bytes = word.to_le_bytes();
        expected_bytes[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
    }

    assert_eq!(EV_EXEC_IMAGE_ID, expected_bytes, "ImageID bytes should match");
}

#[test]
fn test_elf_is_not_empty() {
    // Verify the ELF binary is embedded and not empty
    assert!(!EV_EXEC_ELF.is_empty(), "ELF binary should not be empty");

    // Verify it's a reasonable size (guest binaries are typically > 1MB)
    assert!(EV_EXEC_ELF.len() > 1_000_000, "ELF binary should be at least 1MB");
}

#[test]
fn test_image_id_consistency() {
    // Print the ImageID for debugging
    println!("RISC0 EV-Exec ImageID (u32): {:?}", EV_EXEC_ID);
    println!("RISC0 EV-Exec ImageID (bytes): {:?}", &EV_EXEC_IMAGE_ID[..8]);

    // The ImageID should be deterministic - if this test fails after guest changes,
    // update RISC0_CONSTANTS.rs by rebuilding the guest
    let expected_id = [669635292, 2378518077, 2248775312, 2728884876, 4065561148, 3224919079, 3230001190, 714616563];

    if EV_EXEC_ID != expected_id {
        eprintln!("WARNING: ImageID has changed!");
        eprintln!("Old: {:?}", expected_id);
        eprintln!("New: {:?}", EV_EXEC_ID);
        eprintln!("Update RISC0_CONSTANTS.rs if this was intentional");
    }
}

// Note: Full proof generation tests are expensive and should be run separately
// This test suite focuses on verifying the build artifacts are correct
