use sp1_sdk::{include_elf, HashableKey, Prover, ProverClient};

/// ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_RANGE_EXEC_ELF: &[u8] = include_elf!("evm-range-exec-program");

fn main() {
    let prover = ProverClient::builder().cpu().build();
    let (_, vk) = prover.setup(EVM_RANGE_EXEC_ELF);
    println!("evm-range-exec-program vkey: {}", vk.bytes32());
}
