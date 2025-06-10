use sp1_sdk::{include_elf, HashableKey, Prover, ProverClient};

/// ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_EXEC_ELF: &[u8] = include_elf!("evm-exec-program");

fn main() {
    let prover = ProverClient::builder().cpu().build();
    let (_, vk) = prover.setup(EVM_EXEC_ELF);
    println!("evm-exec-program vkey: {}", vk.bytes32());
}
