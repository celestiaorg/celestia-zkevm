use std::env;
use std::error::Error;

use anyhow::Result;
use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::nmt::{Namespace, NamespaceProof, NamespacedHash};
use clap::Parser;
use eyre::Context;
use nmt_rs::NamespacedSha2Hasher;

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    height: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let height = args.height;

    let namespace_hex = env::var("CELESTIA_NAMESPACE").expect("CELESTIA_NAMESPACE must be set");
    let namespace = Namespace::new_v0(&hex::decode(namespace_hex)?)?;

    let client = Client::new("http://127.0.0.1:26658", None)
        .await
        .context("Failed creating Celestia RPC client")?;

    let header = client.header_get_by_height(height).await?;
    println!("Size of eds (square width): {}", header.dah.square_width());

    let blobs = client.blob_get_all(height, &[namespace]).await?.unwrap();
    println!("Number of blobs in NS: {}", blobs.len());

    let mut modified_header = header.clone();
    modified_header.header.version.app = 3; // Satisfy the rpc error

    let namespace_data = client.share_get_namespace_data(&modified_header, namespace).await?;
    println!(
        "Successfully got {} proofs for namespace data",
        namespace_data.rows.len()
    );

    let mut proofs: Vec<NamespaceProof> = Vec::new();
    for row in namespace_data.rows {
        proofs.push(row.proof);
    }

    let mut roots = Vec::<&NamespacedHash>::new();
    for row_root in header.dah.row_roots() {
        if row_root.contains::<NamespacedSha2Hasher<29>>(namespace.as_bytes().try_into()?) {
            println!(
                "Adding row root. min_ns {}, max_ns {}",
                hex::encode(row_root.min_namespace().0),
                hex::encode(row_root.max_namespace().0)
            );
            roots.push(row_root);
        }
    }

    assert_eq!(
        proofs.len(),
        roots.len(),
        "Number of proofs must match number of namespace inclusion roots"
    );

    let blobdata: Vec<[u8; 512]> = blobs
        .iter()
        .flat_map(|blob| {
            blob.to_shares()
                .unwrap()
                .into_iter()
                .map(|share| share.as_ref().try_into().unwrap())
        })
        .collect();

    println!("Successfully flatmapped blob data to {} raw shares", blobdata.len());

    let mut cursor: usize = 0;
    for (proof, root) in proofs.iter().zip(roots) {
        let start_idx = proof.start_idx() as usize;
        let end_idx = proof.end_idx() as usize;
        println!("Verifying row proof for indices: start {} - end {}", start_idx, end_idx);

        let shares = end_idx - start_idx;

        let end = cursor + shares;

        let raw_leaves = &blobdata[cursor..end];
        proof
            .verify_range(root, raw_leaves, namespace.try_into()?)
            .expect("Failed to veriy proof");

        println!("Successfully verified proof!");
        cursor = end;
    }

    Ok(())
}
