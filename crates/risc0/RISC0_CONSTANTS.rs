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

/// EV-Hyperlane ImageID (32-byte digest) for RISC0
/// This uniquely identifies the ev-hyperlane guest program
pub const RISC0_EV_HYPERLANE_ID: [u32; 8] = [2744069191, 1830777240, 326310898, 4215601095, 1007629970, 1316900118, 4060872798, 1737830579];

/// EV-Hyperlane ImageID as bytes (for compatibility)
pub const RISC0_EV_HYPERLANE_ID_BYTES: [u8; 32] = {
    const ID_U32: [u32; 8] = RISC0_EV_HYPERLANE_ID;
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

/// EV-Range-Exec ImageID (32-byte digest) for RISC0
/// This uniquely identifies the ev-range-exec guest program
pub const RISC0_EV_RANGE_EXEC_ID: [u32; 8] = [1746437846, 1121352631, 310329562, 4084317044, 152303473, 3931257270, 816980004, 2893189446];

/// EV-Range-Exec ImageID as bytes (for compatibility)
pub const RISC0_EV_RANGE_EXEC_ID_BYTES: [u8; 32] = {
    const ID_U32: [u32; 8] = RISC0_EV_RANGE_EXEC_ID;
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
