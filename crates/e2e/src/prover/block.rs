// This endpoint generates a block proof for a range (trusted_height, target_height)
// and wraps it recursively into a single groth16 proof using the ev-range-exec program.

#![allow(unused)]

use std::env;

use alloy_primitives::FixedBytes;
use anyhow::Result;
use ev_prover::{
    config::config::Config,
    prover::programs::block::{AppContext, BlockExecProver},
};

pub async fn prove_blocks(trusted_height: u64, trusted_root: FixedBytes<32>, target_height: u64) -> Result<()> {
    dotenvy::dotenv().ok();
    let prover_mode = env::var("SP1_PROVER").unwrap_or("mock".to_string());
    // parallel mode (network, mock)
    if prover_mode == "mock" || prover_mode == "network" {
        let mut block_exec_prover = BlockExecProver::new(AppContext::from_config(Config::default()).unwrap())?;
        let mut block_prover_lock = block_exec_prover.app.trusted_state.write().await;
        block_prover_lock.height = trusted_height;
        block_prover_lock.root = trusted_root;
        drop(block_prover_lock);
        // run with target height will stop once we have generated a proof for the target height
        block_exec_prover.clone().run(Some(target_height)).await?;

        let latest_block_proof = block_exec_prover.storage.get_latest_block_proof().await?.unwrap();
        println!("Latest block proof height: {:?}", latest_block_proof.celestia_height);
    }
    // synchroneous mode (cuda, cpu)
    else {
        // todo: implement syncroneous mode
    }
    Ok(())
}

pub async fn parallel_prover() -> Result<()> {
    Ok(())
}

pub async fn syncroneous_prover() -> Result<()> {
    Ok(())
}
