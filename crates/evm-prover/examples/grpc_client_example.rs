use evm_prover::proto::celestia::prover::v1::{
    prover_client::ProverClient, AggregateBlockProofsRequest, GetBlockProofRequest, GetBlockProofsInRangeRequest,
    GetLatestBlockProofRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the gRPC server
    let mut client = ProverClient::connect("http://127.0.0.1:50051").await?;

    println!("🚀 Connected to gRPC Prover Service");

    // Example 1: Get a specific block proof
    println!("\n📦 Getting block proof for height 42...");
    match client
        .get_block_proof(GetBlockProofRequest { celestia_height: 42 })
        .await
    {
        Ok(response) => {
            let proof = response.into_inner().proof;
            match proof {
                Some(p) => println!(
                    "✅ Found proof for height {} (created at: {})",
                    p.celestia_height, p.created_at
                ),
                None => println!("❌ No proof data returned"),
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    // Example 2: Get block proofs in a range
    println!("\n📊 Getting block proofs in range 100-105...");
    match client
        .get_block_proofs_in_range(GetBlockProofsInRangeRequest {
            start_height: 100,
            end_height: 105,
        })
        .await
    {
        Ok(response) => {
            let proofs = response.into_inner().proofs;
            println!("✅ Found {} proofs in range", proofs.len());
            for proof in proofs.iter().take(3) {
                // Show first 3
                println!("  - Height {}: {} bytes", proof.celestia_height, proof.proof_data.len());
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    // Example 3: Get the latest block proof
    println!("\n🔍 Getting latest block proof...");
    match client.get_latest_block_proof(GetLatestBlockProofRequest {}).await {
        Ok(response) => {
            let proof = response.into_inner().proof;
            match proof {
                Some(p) => println!(
                    "✅ Latest proof is for height {} (created at: {})",
                    p.celestia_height, p.created_at
                ),
                None => println!("❌ No proofs found"),
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    // Example 4: Aggregate block proofs (this will be slow!)
    println!("\n🔗 Aggregating block proofs for range 100-102...");
    match client
        .aggregate_block_proofs(AggregateBlockProofsRequest {
            start_height: 100,
            end_height: 102,
        })
        .await
    {
        Ok(response) => {
            let aggregated = response.into_inner();
            println!(
                "✅ Generated aggregated proof: {} bytes (public values: {} bytes)",
                aggregated.proof.len(),
                aggregated.public_values.len()
            );
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    println!("\n🎉 Done! All gRPC methods tested.");
    Ok(())
}
