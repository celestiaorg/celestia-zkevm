use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Configure tonic-build
    tonic_build::configure()
        .build_server(false) // We only need client-side code
        .build_client(true)
        .out_dir(&out_dir)
        .compile_protos(
            &["proto/celestia/zkism/v1/tx.proto"],
            &["proto"],
        )?;

    // Tell Cargo to recompile if proto files change
    println!("cargo:rerun-if-changed=proto/");

    Ok(())
}