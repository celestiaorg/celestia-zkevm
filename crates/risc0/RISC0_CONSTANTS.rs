//! RISC0 Program Constants
//!
//! This file contains the ImageIDs for RISC0 guest programs.
//! It is auto-generated from the RISC0 build process and should be included
//! by the main workspace to access RISC0 program identifiers.
//!
//! **IMPORTANT**: This file must be regenerated whenever the guest programs change.
//! Run: `cd crates/risc0 && cargo build --package ev-exec-host`
//!
//! ## Usage in main workspace:
//! ```rust
//! include!("../risc0/RISC0_CONSTANTS.rs");
//! ```

/// EV-Exec ImageID (32-byte digest) for RISC0
/// This uniquely identifies the ev-exec guest program
pub const RISC0_EV_EXEC_ID: [u32; 8] = [669635292, 2378518077, 2248775312, 2728884876, 4065561148, 3224919079, 3230001190, 714616563];

/// EV-Exec ImageID as bytes (for compatibility)
pub const RISC0_EV_EXEC_ID_BYTES: [u8; 32] = {
    const ID_U32: [u32; 8] = RISC0_EV_EXEC_ID;
    let mut bytes = [0u8; 32];
    let mut i = 0;
    while i < 8 {
        let word_bytes = ID_U32[i].to_le_bytes();
        bytes[i * 4] = word_bytes[0];
        bytes[i * 4 + 1] = word_bytes[1];
        bytes[i * 4 + 2] = word_bytes[2];
        bytes[i * 4 + 3] = word_bytes[3];
        i += 1;
    }
    bytes
};

// Note: EV-Hyperlane and EV-Range-Exec ImageIDs will be added here as they are implemented
