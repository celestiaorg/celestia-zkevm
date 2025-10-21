//! RISC0 host/prover for EV execution circuit

// Include the generated methods from the guest program
// This generates constants like: EV_EXEC_ELF, EV_EXEC_ID, etc.
include!(concat!(env!("OUT_DIR"), "/methods.rs"));

// Re-export the guest types for convenience
pub use ev_exec_guest::{Risc0BlockExecInput, BlockExecOutput};

// Export the ImageID as a byte array for compatibility with the prover interface
pub const EV_EXEC_IMAGE_ID: [u8; 32] = {
    const ID_U32: [u32; 8] = EV_EXEC_ID;
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
