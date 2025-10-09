use anyhow::Result;
use clap::{Parser, Subcommand};
use ev_prover::proto::celestia::prover::v1::prover_client::ProverClient;
use ev_prover::proto::celestia::prover::v1::{
    GetBlockProofRequest, GetBlockProofsInRangeRequest, GetLatestBlockProofRequest, GetLatestMembershipProofRequest,
    GetMembershipProofRequest, GetRangeProofsRequest,
};

#[derive(Parser)]
#[command(author, version, about = "Query proof storage via gRPC", long_about = None)]
struct Cli {
    /// gRPC server address
    #[arg(long, default_value = "http://127.0.0.1:50051")]
    server: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a single block proof by height
    BlockProof {
        /// Celestia block height
        #[arg(long)]
        height: u64,
    },
    /// Get block proofs in a range
    BlockProofsInRange {
        /// Start height (inclusive)
        #[arg(long)]
        start: u64,
        /// End height (inclusive)
        #[arg(long)]
        end: u64,
    },
    /// Get the latest block proof
    LatestBlockProof,
    /// Get a membership proof by height
    MembershipProof {
        /// Block height
        #[arg(long)]
        height: u64,
    },
    /// Get the latest membership proof
    LatestMembershipProof,
    /// Get range proofs
    RangeProofs {
        /// Start height
        #[arg(long)]
        start: u64,
        /// End height
        #[arg(long)]
        end: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut client = ProverClient::connect(cli.server.clone()).await?;

    match cli.command {
        Commands::BlockProof { height } => {
            println!("Querying block proof for height {height}...");
            let request = GetBlockProofRequest {
                celestia_height: height,
            };
            let response = client.get_block_proof(request).await?;
            let proof = response.into_inner().proof;

            if let Some(p) = proof {
                println!("✓ Block proof found:");
                println!("  Height: {}", p.celestia_height);
                println!("  Proof size: {} bytes", p.proof_data.len());
                println!("  Public values size: {} bytes", p.public_values.len());
                println!("  Created at: {}", p.created_at);
                println!(
                    "  Proof data (hex): 0x{}...",
                    hex::encode(&p.proof_data[..20.min(p.proof_data.len())])
                );
                println!("  Public values (hex): 0x{}", hex::encode(&p.public_values));
            } else {
                println!("✗ No proof found");
            }
        }
        Commands::BlockProofsInRange { start, end } => {
            println!("Querying block proofs in range [{start}, {end}]...");
            let request = GetBlockProofsInRangeRequest {
                start_height: start,
                end_height: end,
            };
            let response = client.get_block_proofs_in_range(request).await?;
            let proofs = response.into_inner().proofs;

            println!("✓ Found {} block proof(s):", proofs.len());
            for proof in proofs {
                println!(
                    "  - Height: {}, Proof size: {} bytes, Public values size: {} bytes",
                    proof.celestia_height,
                    proof.proof_data.len(),
                    proof.public_values.len()
                );
            }
        }
        Commands::LatestBlockProof => {
            println!("Querying latest block proof...");
            let request = GetLatestBlockProofRequest {};
            let response = client.get_latest_block_proof(request).await?;
            let inner = response.into_inner();

            if inner.has_proof {
                if let Some(p) = inner.proof {
                    println!("✓ Latest block proof found:");
                    println!("  Height: {}", p.celestia_height);
                    println!("  Proof size: {} bytes", p.proof_data.len());
                    println!("  Public values size: {} bytes", p.public_values.len());
                    println!("  Created at: {}", p.created_at);
                } else {
                    println!("✗ No proof data returned");
                }
            } else {
                println!("✗ No proofs in storage");
            }
        }
        Commands::MembershipProof { height } => {
            println!("Querying membership proof for height {height}...");
            let request = GetMembershipProofRequest { height };
            let response = client.get_membership_proof(request).await?;
            let proof = response.into_inner().proof;

            if let Some(p) = proof {
                println!("✓ Membership proof found:");
                println!("  Proof size: {} bytes", p.proof_data.len());
                println!("  Public values size: {} bytes", p.public_values.len());
                println!("  Created at: {}", p.created_at);
                println!(
                    "  Proof data (hex): 0x{}...",
                    hex::encode(&p.proof_data[..20.min(p.proof_data.len())])
                );
                println!("  Public values (hex): 0x{}", hex::encode(&p.public_values));
            } else {
                println!("✗ No proof found");
            }
        }
        Commands::LatestMembershipProof => {
            println!("Querying latest membership proof...");
            let request = GetLatestMembershipProofRequest {};
            let response = client.get_latest_membership_proof(request).await?;
            let inner = response.into_inner();

            if inner.has_proof {
                if let Some(p) = inner.proof {
                    println!("✓ Latest membership proof found:");
                    println!("  Proof size: {} bytes", p.proof_data.len());
                    println!("  Public values size: {} bytes", p.public_values.len());
                    println!("  Created at: {}", p.created_at);
                } else {
                    println!("✗ No proof data returned");
                }
            } else {
                println!("✗ No proofs in storage");
            }
        }
        Commands::RangeProofs { start, end } => {
            println!("Querying range proofs in range [{start}, {end}]...");
            let request = GetRangeProofsRequest {
                start_height: start,
                end_height: end,
            };
            let response = client.get_range_proofs(request).await?;
            let proofs = response.into_inner().proofs;

            println!("✓ Found {} range proof(s):", proofs.len());
            for proof in proofs {
                println!(
                    "  - Range: [{}, {}], Proof size: {} bytes, Public values size: {} bytes",
                    proof.start_height,
                    proof.end_height,
                    proof.proof_data.len(),
                    proof.public_values.len()
                );
            }
        }
    }

    Ok(())
}
