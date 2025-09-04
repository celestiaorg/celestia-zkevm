#![no_main]

use evm_hyperlane_types_sp1::{HyperlaneMessageInputs, HyperlaneMessageOutputs};
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let mut inputs: HyperlaneMessageInputs = sp1_zkvm::io::read::<HyperlaneMessageInputs>();
    inputs.verify();
    sp1_zkvm::io::commit(&HyperlaneMessageOutputs::new(inputs.state_root, inputs.messages));
}
