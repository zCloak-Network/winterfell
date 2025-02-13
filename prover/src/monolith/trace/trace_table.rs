// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use common::{proof::Queries, EvaluationFrame};
use crypto::{Hasher, MerkleTree};
use math::field::StarkField;
use utils::uninit_vector;

#[cfg(feature = "concurrent")]
use rayon::prelude::*;

// TRACE TABLE
// ================================================================================================
pub struct TraceTable<B: StarkField> {
    data: Vec<Vec<B>>,
    blowup: usize,
}

impl<B: StarkField> TraceTable<B> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new trace table from a list of provided register traces.
    pub(super) fn new(data: Vec<Vec<B>>, blowup: usize) -> Self {
        TraceTable { data, blowup }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns number of registers in the trace table.
    pub fn width(&self) -> usize {
        self.data.len()
    }

    /// Returns the number of states in this trace table.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.data[0].len()
    }

    /// Returns blowup factor which was used to extend original trace into this trace.
    pub fn blowup(&self) -> usize {
        self.blowup
    }

    /// Returns value in the specified `register` at the specified `step`.
    pub fn get(&self, register: usize, step: usize) -> B {
        self.data[register][step]
    }

    /// Returns the entire register trace for the register at the specified index.
    #[cfg(test)]
    pub fn get_register(&self, idx: usize) -> &[B] {
        &self.data[idx]
    }

    /// Copies values of all registers at the specified `step` into the `destination` slice.
    pub fn read_row_into(&self, step: usize, row: &mut [B]) {
        for (register, value) in self.data.iter().zip(row.iter_mut()) {
            *value = register[step];
        }
    }

    /// Reads current and next rows from the execution trace table into the specified frame.
    pub fn read_frame_into(&self, lde_step: usize, frame: &mut EvaluationFrame<B>) {
        // at the end of the trace, next state wraps around and we read the first step again
        let next_lde_step = (lde_step + self.blowup()) % self.len();

        self.read_row_into(lde_step, &mut frame.current);
        self.read_row_into(next_lde_step, &mut frame.next);
    }

    // TRACE COMMITMENT
    // --------------------------------------------------------------------------------------------
    /// Builds a Merkle tree out of trace table rows (hash of each row becomes a leaf in the tree).
    pub fn build_commitment<H: Hasher>(&self) -> MerkleTree {
        let hash_fn = H::hash_fn();
        // allocate vector to store row hashes
        let mut hashed_states = uninit_vector::<[u8; 32]>(self.len());

        // iterate though table rows, hashing each row; the hashing is done by first copying
        // the state into trace_state buffer to avoid unneeded allocations, and then by applying
        // the hash function to the buffer.
        #[cfg(feature = "concurrent")]
        {
            let batch_size = hashed_states.len() / rayon::current_num_threads().next_power_of_two();
            hashed_states
                .par_chunks_mut(batch_size)
                .enumerate()
                .for_each(|(batch_idx, hashed_states_batch)| {
                    let offset = batch_idx * batch_size;
                    let mut trace_state = vec![B::ZERO; self.width()];
                    for (i, row_hash) in hashed_states_batch.iter_mut().enumerate() {
                        self.read_row_into(i + offset, &mut trace_state);
                        hash_fn(B::elements_as_bytes(&trace_state), row_hash);
                    }
                });
        }

        #[cfg(not(feature = "concurrent"))]
        {
            let mut trace_state = vec![B::ZERO; self.width()];
            for (i, row_hash) in hashed_states.iter_mut().enumerate() {
                self.read_row_into(i, &mut trace_state);
                hash_fn(B::elements_as_bytes(&trace_state), row_hash);
            }
        }

        // build Merkle tree out of hashed rows
        MerkleTree::new(hashed_states, hash_fn)
    }

    // QUERY TRACE
    // --------------------------------------------------------------------------------------------
    /// Returns trace table rows at the specified positions along with Merkle authentication paths
    /// from the `commitment` root to these rows.
    pub fn query(&self, commitment: MerkleTree, positions: &[usize]) -> Queries {
        assert_eq!(
            self.len(),
            commitment.leaves().len(),
            "inconsistent trace table commitment"
        );

        // allocate memory for queried trace states
        let mut trace_states = Vec::with_capacity(positions.len());

        // copy values from the trace table at the specified positions into rows
        // and append the rows to trace_states
        for &i in positions.iter() {
            let row = self.data.iter().map(|r| r[i]).collect();
            trace_states.push(row);
        }

        // build Merkle authentication paths to the leaves specified by positions
        let trace_proof = commitment.prove_batch(&positions);

        Queries::new(trace_proof, trace_states)
    }
}
