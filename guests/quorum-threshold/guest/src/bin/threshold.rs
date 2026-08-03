use quorum_circuit::{evaluate, ThresholdWitness};
use risc0_zkvm::guest::env;

fn main() {
    let witness: ThresholdWitness = env::read();
    let journal = evaluate(&witness).expect("invalid threshold witness");
    env::commit(&journal);
}
