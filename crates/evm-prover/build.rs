use std::path::PathBuf;

use sp1_build::build_program_with_args;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect all .proto files recursively under /proto/
    let proto_files: Vec<PathBuf> = WalkDir::new("../../proto")
        .into_iter()
        .filter_map(|entry| {
            let path = entry.ok()?.path().to_path_buf();
            if path.extension().is_some_and(|ext| ext == "proto") {
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

    build_program_with_args("../sp1/evm-exec/program", Default::default());
    build_program_with_args("../sp1/evm-range-exec/program", Default::default());
    build_program_with_args("../sp1/evm-hyperlane/program", Default::default());

    Ok(())
}
