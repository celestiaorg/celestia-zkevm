use std::path::PathBuf;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect all .proto files recursively under /proto/
    let proto_files: Vec<PathBuf> = WalkDir::new("../../proto")
        .into_iter()
        .filter_map(|entry| {
            let path = entry.ok()?.path().to_path_buf();
            if path.extension().map_or(false, |ext| ext == "proto") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path("src/proto/descriptor.bin")
        .out_dir("src/proto")
        .compile(&proto_files, &["../../proto"])?;

    // This build.rs can be extended to compile the SP1 programs and output the elfs to a standard path
    // from which we can then consume them: e.g. output to evm-prover/artifacts or evm-prover/elf
    // consume from prover/prover.rs with include_elf!("../../artifacts/evm-exec-elf")
    // ref: https://docs.succinct.xyz/docs/sp1/writing-programs/compiling#build-script

    Ok(())
}
