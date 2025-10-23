use std::fs;

use sp1_sdk::{include_elf, HashableKey, Prover, ProverClient};

/// ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_EXEC_ELF: &[u8] = include_elf!("ev-combined-program");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prover = ProverClient::builder().cpu().build();
    let (_, vk) = prover.setup(EV_EXEC_ELF);

    let path = "testdata/vkeys/ev-combined-vkey-hash";
    fs::write(path, vk.bytes32())?;
    println!("ev-combined-program vkey: {}", vk.bytes32());

    let encoded = bincode::serialize(&vk)?;
    let path = "testdata/vkeys/ev-combined-vkey.bin";
    fs::write(path, encoded)?;
    println!("successfully wrote vkey to: {path}");

    let path = "elfs/ev-combined-elf";
    fs::write(path, EV_EXEC_ELF)?;
    println!("successfully wrote elf to: {path}");

    Ok(())
}
