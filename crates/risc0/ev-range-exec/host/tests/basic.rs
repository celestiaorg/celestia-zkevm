//! Basic integration test for RISC0 EV-Range-Exec circuit

use ev_range_exec_host::{EV_RANGE_EXEC_ELF, EV_RANGE_EXEC_ID, EV_RANGE_EXEC_IMAGE_ID};

#[test]
fn test_image_id_is_valid() {
    // Verify the ImageID is not all zeros
    assert_ne!(EV_RANGE_EXEC_ID, [0u32; 8], "ImageID should not be all zeros");

    // Verify the ImageID bytes are correctly derived
    let mut expected_bytes = [0u8; 32];
    for (i, &word) in EV_RANGE_EXEC_ID.iter().enumerate() {
        let bytes = word.to_le_bytes();
        expected_bytes[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
    }

    assert_eq!(EV_RANGE_EXEC_IMAGE_ID, expected_bytes, "ImageID bytes should match");
}

#[test]
fn test_elf_is_not_empty() {
    // Verify the ELF binary is embedded and not empty
    assert!(!EV_RANGE_EXEC_ELF.is_empty(), "ELF binary should not be empty");

    // Verify it's a reasonable size (guest binaries are typically > 100KB)
    assert!(EV_RANGE_EXEC_ELF.len() > 100_000, "ELF binary should be at least 100KB");
}

#[test]
fn test_image_id_consistency() {
    // Print the ImageID for debugging
    println!("RISC0 EV-Range-Exec ImageID (u32): {:?}", EV_RANGE_EXEC_ID);
    println!("RISC0 EV-Range-Exec ImageID (bytes): {:?}", &EV_RANGE_EXEC_IMAGE_ID[..8]);

    // The ImageID should be deterministic
    let expected_id = [1746437846, 1121352631, 310329562, 4084317044, 152303473, 3931257270, 816980004, 2893189446];

    if EV_RANGE_EXEC_ID != expected_id {
        eprintln!("WARNING: ImageID has changed!");
        eprintln!("Old: {:?}", expected_id);
        eprintln!("New: {:?}", EV_RANGE_EXEC_ID);
        eprintln!("Update RISC0_CONSTANTS.rs if this was intentional");
    }
}
