fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only build SP1 programs if the sp1 feature is enabled
    #[cfg(feature = "sp1")]
    {
        use sp1_build::build_program_with_args;

        build_program_with_args("../sp1/ev-exec/program", Default::default());
        build_program_with_args("../sp1/ev-range-exec/program", Default::default());
        build_program_with_args("../sp1/ev-hyperlane/program", Default::default());
    }

    // RISC0 programs are built separately via their own build.rs files
    // and included via the risc0-build crate in each host package

    Ok(())
}
