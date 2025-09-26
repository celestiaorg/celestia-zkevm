use std::fs;

use sp1_sdk::{include_elf, HashableKey, Prover, ProverClient};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EV_HYPERLANE_ELF: &[u8] = include_elf!("ev-hyperlane-program");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prover = ProverClient::builder().cpu().build();
    let (_, vk) = prover.setup(EV_HYPERLANE_ELF);

    let path = "testdata/vkeys/ev-hyperlane-vkey-hash";
    fs::write(path, vk.bytes32())?;
    println!("ev-hyperlane-program vkey: {}", vk.bytes32());

    let encoded = bincode::serialize(&vk)?;
    let path = "testdata/vkeys/ev-hyperlane-vkey.bin";
    fs::write(path, encoded)?;
    println!("successfully wrote vkey to: {path}");

    let path = "elfs/ev-hyperlane-elf";
    fs::write(path, EV_HYPERLANE_ELF)?;
    println!("successfully wrote elf to: {path}");

    Ok(())
}
