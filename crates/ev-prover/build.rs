use sp1_build::build_program_with_args;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    build_program_with_args("../sp1/ev-exec/program", Default::default());
    build_program_with_args("../sp1/ev-range-exec/program", Default::default());
    build_program_with_args("../sp1/ev-hyperlane/program", Default::default());

    Ok(())
}
