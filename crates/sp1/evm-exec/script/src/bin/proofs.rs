use std::env;
use std::error::Error;

use anyhow::Result;
use celestia_rpc::{BlobClient, Client, HeaderClient};
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

    println!("Number of rows containing data for NS: {}", roots.len());

    let blobs = client.blob_get_all(height, &[namespace]).await?.unwrap();
    println!("Number of blobs in NS: {}", blobs.len());

    // for blob in blobs {
    //     let nmt_proofs = client.blob_get_proof(height, namespace, blob.commitment).await?;

    //     println!("len of nmt proofs for blob: {}", nmt_proofs.len());
    //     // assert_eq!(
    //     //     nmt_proofs.len(),
    //     //     roots.len(),
    //     //     "Number of proofs must match number of namespace inclusion roots"
    //     // );

    //     for (proof, root) in nmt_proofs.iter().zip(&roots) {
    //         let raw_leaves: Vec<[u8; 512]> = blob
    //             .to_shares()
    //             .unwrap()
    //             .into_iter()
    //             .map(|share| share.as_ref().try_into().unwrap())
    //             .collect();

    //         println!("start idx: {}, end idx: {}", proof.start_idx(), proof.end_idx());
    //         println!("num leaves: {}", raw_leaves.len());

    //         proof
    //             .verify_range(root, &raw_leaves, namespace.try_into()?)
    //             .expect("Failed to veriy proof");
    //     }
    // }

    let mut raw_leaves: Vec<[u8; 512]> = Vec::new();
    let mut proofs: Vec<NamespaceProof> = Vec::new();
    for blob in blobs.clone() {
        println!(
            "Getting proofs for blob commitment: {}, shares length: {}",
            hex::encode(blob.commitment.hash()),
            blob.shares_len(),
        );

        let nmt_proofs = client.blob_get_proof(height, namespace, blob.commitment).await?;
        println!("Number of NMT proofs for blob: {}", nmt_proofs.len());
        proofs.extend(nmt_proofs);

        let mut leaves: Vec<[u8; 512]> = blob
            .to_shares()
            .unwrap()
            .into_iter()
            .map(|share| share.as_ref().try_into().unwrap())
            .collect();

        raw_leaves.append(&mut leaves);
    }

    // assert_eq!(
    //     nmt_proofs.len(),
    //     roots.len(),
    //     "Number of proofs must match number of namespace inclusion roots"
    // );

    // let mut cursor: usize = 0;
    // for (proof, root) in proofs.iter().zip(roots) {
    //     let start_idx = proof.start_idx() as usize;
    //     let end_idx = proof.end_idx() as usize;
    //     let shares = end_idx - start_idx;

    //     println!("start idx: {}, end idx: {}", start_idx, end_idx);

    //     let end = cursor + shares;

    //     proof
    //         .verify_range(root, &raw_leaves[cursor..end], namespace.try_into()?)
    //         .expect("Failed to verify proof");

    //     cursor = end;

    //     println!("successfully verified")
    // }

    let first_proof = &proofs[0];
    let first_root = roots[0];

    let first_leaves: Vec<[u8; 512]> = blobs[0]
        .to_shares()
        .unwrap()
        .into_iter()
        .map(|share| share.as_ref().try_into().unwrap())
        .collect();

    println!("blobs[0].index: {}", blobs[0].index.unwrap());

    let second_leaves: Vec<[u8; 512]> = blobs[1]
        .to_shares()
        .unwrap()
        .into_iter()
        .map(|share| share.as_ref().try_into().unwrap())
        .collect();

    let mut combined_leaves: Vec<[u8; 512]> = Vec::new();
    combined_leaves.extend(first_leaves.iter().copied()); // All of first_leaves

    if let Some(first_third) = second_leaves.first() {
        combined_leaves.push(*first_third); // First element of second_leaves
    }

    println!("length of combined leaves {}", combined_leaves.len());
    println!(
        "end {} - start {} = {}",
        first_proof.end_idx(),
        first_proof.start_idx(),
        first_proof.end_idx() - first_proof.start_idx()
    );

    first_proof
        .verify_range(&first_root, &combined_leaves, namespace.try_into()?)
        .expect("failed to verify nmt proof");

    println!("blobs[1].index: {}", blobs[1].index.unwrap());
    let second_proof = &proofs[1];
    println!(
        "end {} - start {} = {}",
        second_proof.end_idx(),
        second_proof.start_idx(),
        second_proof.end_idx() - second_proof.start_idx()
    );

    Ok(())
}
