//! RISC0 guest binary for EV-Hyperlane message verification

use risc0_zkvm::guest::env;

pub fn main() {
    // Read inputs from host
    let mut inputs = env::read();

    // Verify using the library function
    let output = ev_hyperlane_guest::verify_and_commit(inputs);

    // Commit outputs to journal
    env::commit(&output);
}
