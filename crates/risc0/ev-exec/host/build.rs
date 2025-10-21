fn main() {
    // The risc0_build tool automatically discovers guest methods in sibling directories
    // It looks for packages ending in "-guest" within the same parent directory
    risc0_build::embed_methods();
}
