// Risc0 host/prover for EV execution circuit

// Include the generated methods from the guest program
// This generates constants like: EV_EXEC_GUEST_ID, EV_EXEC_GUEST_PATH, etc.
risc0_zkvm::guest::host::include_methods!();

// Re-export the guest module for compatibility
pub use ev_exec_guest::*;

// Export the ImageID for use in the prover
// The risc0 macro generates EV_EXEC_GUEST_ID as [u32; 8]
// We need to convert it to &[u8] for compatibility with the prover interface
pub const EV_EXEC_ID: [u8; 32] = {
    const ID_U32: [u32; 8] = EV_EXEC_GUEST_ID;
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
