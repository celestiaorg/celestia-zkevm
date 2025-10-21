//! RISC0 guest binary for EV execution circuit

use risc0_zkvm::guest::env;

pub fn main() {
    env::log("Reading inputs");
    let input = env::read();

    env::log("Verifying and executing");
    let output = ev_exec_guest::verify_and_execute(input).expect("Block execution verification failed");

    env::log("Committing output");
    env::commit(&output);
}
