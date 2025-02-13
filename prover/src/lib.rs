// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

mod monolith;
pub use monolith::{prove, ExecutionTrace, ExecutionTraceFragment};

mod channel;

pub use common::{
    proof::StarkProof, Air, Assertion, ComputationContext, EvaluationFrame, FieldExtension,
    HashFunction, ProofOptions, TraceInfo, TransitionConstraintDegree, TransitionConstraintGroup,
};
pub use crypto;
pub use math;

#[cfg(test)]
pub mod tests;
