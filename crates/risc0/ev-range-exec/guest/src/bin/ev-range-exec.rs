//! RISC0 guest binary for EV-Range-Exec recursive proof verification

use risc0_zkvm::guest::env;

pub fn main() {
    // Read inputs from host
    let inputs = env::read();

    // Verify and aggregate using the library function
    let output = ev_range_exec_guest::verify_and_aggregate(inputs);

    // Commit outputs to journal
    env::commit(&output);
}
