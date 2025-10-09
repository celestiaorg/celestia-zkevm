/// End-to-end test that queries stored proofs from the gRPC server
/// This test assumes that:
/// 1. The ev-prover gRPC server is running on localhost:50051
/// 2. Some proofs have been generated and stored (e.g., by running the block prover)
use anyhow::Result;
use ev_prover::proto::celestia::prover::v1::prover_client::ProverClient;
use ev_prover::proto::celestia::prover::v1::{
    GetBlockProofRequest, GetBlockProofsInRangeRequest, GetLatestBlockProofRequest,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== E2E Test: Query Stored Proofs ===\n");

    let server_addr = "http://127.0.0.1:50051";
    println!("Connecting to gRPC server at {server_addr}...");

    let mut client = ProverClient::connect(server_addr).await?;
    println!("✓ Connected to gRPC server\n");

    // Test 1: Get latest block proof
    println!("Test 1: Get latest block proof");
    println!("---");
    match client.get_latest_block_proof(GetLatestBlockProofRequest {}).await {
        Ok(response) => {
            let inner = response.into_inner();
            if inner.has_proof {
                if let Some(proof) = inner.proof {
                    println!("✓ Found latest block proof:");
                    println!("  Height: {}", proof.celestia_height);
                    println!("  Proof size: {} bytes", proof.proof_data.len());
                    println!("  Public values size: {} bytes", proof.public_values.len());
                    println!("  Created at: {}", proof.created_at);

                    // Use this height for subsequent tests
                    let latest_height = proof.celestia_height;

                    // Test 2: Get specific block proof by height
                    println!("\nTest 2: Get block proof by height ({latest_height})");
                    println!("---");
                    match client
                        .get_block_proof(GetBlockProofRequest {
                            celestia_height: latest_height,
                        })
                        .await
                    {
                        Ok(resp) => {
                            if let Some(p) = resp.into_inner().proof {
                                println!("✓ Successfully retrieved block proof:");
                                println!("  Height: {}", p.celestia_height);
                                println!("  Proof size: {} bytes", p.proof_data.len());
                                assert_eq!(p.celestia_height, latest_height);
                            } else {
                                println!("✗ No proof data returned");
                            }
                        }
                        Err(e) => {
                            println!("✗ Failed to get block proof: {e}");
                        }
                    }

                    // Test 3: Get block proofs in range
                    if latest_height > 0 {
                        let start_height = if latest_height >= 5 { latest_height - 4 } else { 1 };
                        let end_height = latest_height;

                        println!("\nTest 3: Get block proofs in range [{start_height}, {end_height}]");
                        println!("---");
                        match client
                            .get_block_proofs_in_range(GetBlockProofsInRangeRequest {
                                start_height,
                                end_height,
                            })
                            .await
                        {
                            Ok(resp) => {
                                let proofs = resp.into_inner().proofs;
                                println!("✓ Found {} block proof(s) in range:", proofs.len());
                                for p in &proofs {
                                    println!(
                                        "  - Height: {}, Proof size: {} bytes",
                                        p.celestia_height,
                                        p.proof_data.len()
                                    );
                                }

                                // Verify proofs are within range
                                for p in &proofs {
                                    assert!(
                                        p.celestia_height >= start_height && p.celestia_height <= end_height,
                                        "Proof height {} not in range [{}, {}]",
                                        p.celestia_height,
                                        start_height,
                                        end_height
                                    );
                                }
                            }
                            Err(e) => {
                                println!("✗ Failed to get block proofs in range: {e}");
                            }
                        }
                    }
                } else {
                    println!("✗ No proof data returned");
                }
            } else {
                println!("⚠ No proofs in storage yet");
                println!("  Run the block prover first to generate some proofs");
            }
        }
        Err(e) => {
            println!("✗ Failed to connect or query: {e}");
            println!("  Make sure the ev-prover gRPC server is running");
            return Err(e.into());
        }
    }

    println!("\n=== All tests completed ===");
    Ok(())
}
