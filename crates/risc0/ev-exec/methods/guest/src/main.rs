use risc0_zkvm::guest::env;

use ev_zkevm_types::programs::block::BlockExecInput;
fn main() {
    println!("cycle-tracker-report-start: deserialize inputs");
    let inputs: BlockExecInput = env::read::<BlockExecInput>();
    let output = ev_exec::verify_ev_exec(inputs).expect("failed to verify ev exec");
    env::commit(&output);
    println!("cycle-tracker-report-end: commit public outputs");
}
