use std::collections::HashMap;

fn main() {
    let mut options = HashMap::new();
    options.insert("../guest", risc0_build::GuestOptions::default());
    risc0_build::embed_methods_with_options(options);
}
