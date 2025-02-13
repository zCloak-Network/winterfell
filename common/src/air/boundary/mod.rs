// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::{Assertion, ConstraintDivisor};
use crypto::RandomElementGenerator;
use math::{
    fft,
    field::{FieldElement, StarkField},
    polynom,
};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

// BOUNDARY CONSTRAINT GROUP
// ================================================================================================

/// A group of boundary constraints all having the same divisor.
#[derive(Debug, Clone)]
pub struct BoundaryConstraintGroup<B, E>
where
    B: StarkField,
    E: FieldElement + From<B>,
{
    constraints: Vec<BoundaryConstraint<B, E>>,
    divisor: ConstraintDivisor<B>,
    degree_adjustment: u32,
}

impl<B, E> BoundaryConstraintGroup<B, E>
where
    B: StarkField,
    E: FieldElement + From<B>,
{
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    pub fn new(
        divisor: ConstraintDivisor<B>,
        trace_poly_degree: usize,
        composition_degree: usize,
    ) -> Self {
        // We want to make sure that once we divide a constraint polynomial by its divisor, the
        // degree of the resulting polynomials will be exactly equal to the composition_degree.
        // Boundary constraint degree is always deg(trace). So, the adjustment degree is simply:
        // deg(composition) + deg(divisor) - deg(trace)
        let target_degree = composition_degree + divisor.degree();
        let degree_adjustment = (target_degree - trace_poly_degree) as u32;

        BoundaryConstraintGroup {
            constraints: Vec::new(),
            divisor,
            degree_adjustment,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a list of boundary constraints in this group.
    pub fn constraints(&self) -> &[BoundaryConstraint<B, E>] {
        &self.constraints
    }

    /// Returns a divisor applicable to all boundary constraints in this group.
    pub fn divisor(&self) -> &ConstraintDivisor<B> {
        &self.divisor
    }

    /// Returns a degree adjustment factor for all boundary constraints in this group.
    pub fn degree_adjustment(&self) -> u32 {
        self.degree_adjustment
    }

    // Returns degree of the largest constraint polynomial in this group.
    pub fn max_poly_degree(&self) -> usize {
        let mut poly_size = 0;
        for constraint in self.constraints.iter() {
            if constraint.poly().len() > poly_size {
                poly_size = constraint.poly().len();
            }
        }
        poly_size - 1
    }

    // PUBLIC METHODS
    // --------------------------------------------------------------------------------------------

    /// Creates a new boundary constraint from the specified assertion and adds it to the group.
    pub fn add<R: RandomElementGenerator>(
        &mut self,
        assertion: Assertion<B>,
        inv_g: B,
        twiddle_map: &mut HashMap<usize, Vec<B>>,
        coeff_prng: &mut R,
    ) {
        self.constraints.push(BoundaryConstraint::new(
            assertion,
            inv_g,
            twiddle_map,
            coeff_prng,
        ));
    }

    /// Evaluates all constraints in this group at the specified point `x`, and merges the
    /// results into a single value by computing a random linear combination of the results.
    pub fn evaluate_at(&self, state: &[E], x: E, xp: E) -> E {
        let mut result = E::ZERO;
        for constraint in self.constraints().iter() {
            let evaluation = constraint.evaluate_at(x, state[constraint.register()]);
            result += evaluation * (constraint.cc().0 + constraint.cc().1 * xp);
        }
        result
    }
}

// BOUNDARY CONSTRAINT
// ================================================================================================

/// Describes the numerator portion of a boundary constraint.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BoundaryConstraint<B, E>
where
    B: StarkField,
    E: FieldElement + From<B>,
{
    register: usize,
    poly: Vec<B>,
    poly_offset: (usize, B),
    cc: (E, E),
}

impl<B, E> BoundaryConstraint<B, E>
where
    B: StarkField,
    E: FieldElement + From<B>,
{
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new boundary constraint from the specified assertion.
    pub fn new<R: RandomElementGenerator>(
        assertion: Assertion<B>,
        inv_g: B,
        twiddle_map: &mut HashMap<usize, Vec<B>>,
        coeff_prng: &mut R,
    ) -> Self {
        // build a polynomial which evaluates to constraint values at asserted steps; for
        // single-value assertions we use the value as constant coefficient of degree 0
        // polynomial; but for multi-value assertions, we need to interpolate the values
        // into a polynomial using inverse FFT
        let mut poly_offset = (0, B::ONE);
        let mut poly = assertion.values;
        if poly.len() > 1 {
            // get the twiddles from the map; if twiddles for this domain haven't been built
            // yet, build them and add them to the map
            let inv_twiddles = twiddle_map
                .entry(poly.len())
                .or_insert_with(|| fft::get_inv_twiddles(poly.len()));
            // interpolate the values into a polynomial
            fft::interpolate_poly(&mut poly, &inv_twiddles);
            if assertion.first_step != 0 {
                // if the assertions don't fall on the steps which are powers of two, we can't
                // use FFT to interpolate the values into a polynomial. This would make such
                // assertions quite impractical. To get around this, we still use FFT to build
                // the polynomial, but then we evaluate it as f(x * offset) instead of f(x)
                let x_offset = inv_g.exp((assertion.first_step as u64).into());
                poly_offset = (assertion.first_step, x_offset);
            }
        }

        BoundaryConstraint {
            register: assertion.register,
            poly,
            poly_offset,
            cc: coeff_prng.draw_pair(),
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns index of the register against which this constraint applies.
    pub fn register(&self) -> usize {
        self.register
    }

    /// Returns constraint polynomial for this constraint.
    pub fn poly(&self) -> &[B] {
        &self.poly
    }

    /// Returns offset by which we need to shift the domain before evaluating this constraint.
    /// The offset is returned as a tuple describing both, the number of steps by which the
    /// domain needs to be shifted, and field element by which a domain element needs to be
    /// multiplied to achieve the desired shift.
    pub fn poly_offset(&self) -> (usize, B) {
        self.poly_offset
    }

    /// Returns composition coefficients for this constraint.
    pub fn cc(&self) -> &(E, E) {
        &self.cc
    }

    // PUBLIC METHODS
    // --------------------------------------------------------------------------------------------

    /// Evaluates this constraint at the specified point `x` by computing trace_value - P(x).
    /// trace_value is assumed to be evaluation of a trace polynomial at `x`.
    pub fn evaluate_at(&self, x: E, trace_value: E) -> E {
        let assertion_value = if self.poly.len() == 1 {
            // if constraint polynomial consists of just a constant, use that constant
            E::from(self.poly[0])
        } else {
            // otherwise, we need to evaluate the polynomial at `x`; but first do the following:
            // 1. for assertions which don't fall on steps that are powers of two, we need to
            //    evaluate assertion polynomial at x * offset (instead of just x)
            // 2. map the coefficients of the polynomial into the evaluation field. If we are
            //    working in the base field, this has not effect; but if we are working in an
            //    extension field, coefficients of the polynomial are mapped from the base
            //    field into the extension field.
            let x = x * E::from(self.poly_offset.1);
            polynom::eval(&self.poly, x)
        };
        // subtract assertion value from trace value
        trace_value - assertion_value
    }
}
