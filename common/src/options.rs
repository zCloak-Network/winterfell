// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use fri::FriOptions;
use math::field::StarkField;
use serde::{Deserialize, Serialize};

// TYPES AND INTERFACES
// ================================================================================================

#[repr(u8)]
#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum FieldExtension {
    None = 1,
    Quadratic = 2,
}

#[repr(u8)]
#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum HashFunction {
    Blake3_256 = 1,
    Sha3_256 = 2,
}

// TODO: validate field values on de-serialization
#[derive(Clone, Serialize, Deserialize)]
pub struct ProofOptions {
    num_queries: u8,
    blowup_factor: u8, // stored as power of 2
    grinding_factor: u8,
    hash_fn: HashFunction,
    field_extension: FieldExtension,
}

// PROOF OPTIONS IMPLEMENTATION
// ================================================================================================
impl ProofOptions {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    /// Returns new ProofOptions struct constructed from the specified parameters, which must
    /// comply with the following:
    /// * num_queries must be an integer between 1 and 128;
    /// * blowup_factor must be an integer which is a power of two between 4 and 256;
    /// * grinding_factor must be an integer between 0 and 32;
    /// * hash_fn must be blake3 or sha3 functions from crypto crate;
    pub fn new(
        num_queries: usize,
        blowup_factor: usize,
        grinding_factor: u32,
        hash_fn: HashFunction,
        field_extension: FieldExtension,
    ) -> ProofOptions {
        assert!(num_queries > 0, "num_queries must be greater than 0");
        assert!(num_queries <= 128, "num_queries cannot be greater than 128");

        assert!(
            blowup_factor.is_power_of_two(),
            "blowup_factor must be a power of 2"
        );
        assert!(blowup_factor >= 4, "blowup_factor cannot be smaller than 4");
        assert!(
            blowup_factor <= 256,
            "blowup_factor cannot be greater than 256"
        );

        assert!(
            grinding_factor <= 32,
            "grinding factor cannot be greater than 32"
        );

        ProofOptions {
            num_queries: num_queries as u8,
            blowup_factor: blowup_factor.trailing_zeros() as u8,
            grinding_factor: grinding_factor as u8,
            hash_fn,
            field_extension,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns number of queries for a STARK proof. This directly impacts proof soundness as each
    /// additional query adds roughly log2(lde_domain_size / constraint_evaluation_domain_size)
    /// bits of security to a proof. However, each additional query also increases proof size.
    pub fn num_queries(&self) -> usize {
        self.num_queries as usize
    }

    /// Returns trace blowup factor for a STARK proof (i.e. a factor by which the execution
    /// trace is extended). This directly impacts proof soundness as each query adds roughly
    /// log2(lde_domain_size / constraint_evaluation_domain_size) bits of security to a proof.
    /// However, higher blowup factors also increases prover runtime - e.g. doubling blowup
    /// factor roughly doubles prover time.
    pub fn blowup_factor(&self) -> usize {
        1 << (self.blowup_factor as usize)
    }

    /// Returns query seed grinding factor for a STARK proof. Grinding applies Proof-of-Work
    /// to the query position seed. An honest prover needs to perform this work only once,
    /// while a dishonest prover will need to perform it every time they try to change a
    /// commitment. Thus, higher grinding factor makes it more difficult to forge a STARK
    /// proof. However, setting grinding factor too high (e.g. higher than 20) will adversely
    /// affect prover time.
    pub fn grinding_factor(&self) -> u32 {
        self.grinding_factor as u32
    }

    /// Returns a hash functions to be used during STARK proof construction. Security of a
    /// STARK proof is bounded by collision resistance of the used hash function.
    pub fn hash_fn(&self) -> HashFunction {
        self.hash_fn
    }

    /// Returns a value indicating whether an extension field should be used for the composition
    /// polynomial. Using a field extension increases maximum security level of a proof, but
    /// also has non-negligible impact on prover performance.
    pub fn field_extension(&self) -> FieldExtension {
        self.field_extension
    }

    /// Returns the offset by which the low-degree extension domain is shifted in relation to the
    /// trace domain. Currently, this is hard-coded to the generator of the underlying base field.
    pub fn domain_offset<B: StarkField>(&self) -> B {
        B::GENERATOR
    }

    /// Returns options for FRI protocol instantiated with parameters from this proof options.
    pub fn to_fri_options<B: StarkField>(&self) -> FriOptions<B> {
        FriOptions::new(self.blowup_factor(), self.domain_offset())
    }
}

// FIELD EXTENSION IMPLEMENTATION
// ================================================================================================

impl FieldExtension {
    /// Returns `true` if this field extension is set to None.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}
