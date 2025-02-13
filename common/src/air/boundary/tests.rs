// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::{
    super::tests::{build_prng, build_sequence_poly},
    Assertion, BoundaryConstraint,
};
use crypto::{hash, DefaultRandomElementGenerator, RandomElementGenerator};
use math::{
    field::{f128::BaseElement, FieldElement, StarkField},
    polynom,
    utils::log2,
};
use std::collections::HashMap;

// BOUNDARY CONSTRAINT TESTS
// ================================================================================================

#[test]
fn boundary_constraint_from_single_assertion() {
    let mut test_prng = build_prng();
    let (inv_g, mut twiddle_map, mut prng) = build_constraint_params(16);

    // constraint should be built correctly for register 0, step 0
    let value = BaseElement::rand();
    let assertion = Assertion::single(0, 0, value);
    let constraint = BoundaryConstraint::<BaseElement, BaseElement>::new(
        assertion,
        inv_g,
        &mut twiddle_map,
        &mut prng,
    );
    assert_eq!(0, constraint.register());
    assert_eq!(vec![value], constraint.poly());
    assert_eq!((0, BaseElement::ONE), constraint.poly_offset());
    assert_eq!(test_prng.draw_pair::<BaseElement>(), constraint.cc);

    // single value constraints should evaluate to trace_value - value
    let trace_value = BaseElement::rand();
    assert_eq!(
        trace_value - value,
        constraint.evaluate_at(BaseElement::rand(), trace_value)
    );

    // constraint is build correctly for register 1 step 8
    let value = BaseElement::rand();
    let assertion = Assertion::single(1, 8, value);
    let constraint = BoundaryConstraint::<BaseElement, BaseElement>::new(
        assertion,
        inv_g,
        &mut twiddle_map,
        &mut prng,
    );
    assert_eq!(1, constraint.register());
    assert_eq!(vec![value], constraint.poly());
    assert_eq!((0, BaseElement::ONE), constraint.poly_offset());
    assert_eq!(test_prng.draw_pair::<BaseElement>(), constraint.cc);

    // single value constraints should evaluate to trace_value - value
    let trace_value = BaseElement::rand();
    assert_eq!(
        trace_value - value,
        constraint.evaluate_at(BaseElement::rand(), trace_value)
    );

    // twiddle map was not touched
    assert!(twiddle_map.is_empty());
}

#[test]
fn boundary_constraint_from_periodic_assertion() {
    let mut test_prng = build_prng();
    let (inv_g, mut twiddle_map, mut prng) = build_constraint_params(16);

    // constraint should be built correctly for register 0, step 0, stride 4
    let value = BaseElement::rand();
    let assertion = Assertion::periodic(0, 0, 4, value);
    let constraint = BoundaryConstraint::<BaseElement, BaseElement>::new(
        assertion,
        inv_g,
        &mut twiddle_map,
        &mut prng,
    );
    assert_eq!(0, constraint.register());
    assert_eq!(vec![value], constraint.poly());
    assert_eq!((0, BaseElement::ONE), constraint.poly_offset());
    assert_eq!(test_prng.draw_pair::<BaseElement>(), constraint.cc);

    // periodic value constraints should evaluate to trace_value - value
    let trace_value = BaseElement::rand();
    assert_eq!(
        trace_value - value,
        constraint.evaluate_at(BaseElement::rand(), trace_value)
    );

    // constraint should be built correctly for register 2, first step 3, stride 8
    let value = BaseElement::rand();
    let assertion = Assertion::periodic(2, 3, 8, value);
    let constraint = BoundaryConstraint::<BaseElement, BaseElement>::new(
        assertion,
        inv_g,
        &mut twiddle_map,
        &mut prng,
    );
    assert_eq!(2, constraint.register());
    assert_eq!(vec![value], constraint.poly());
    assert_eq!((0, BaseElement::ONE), constraint.poly_offset());
    assert_eq!(test_prng.draw_pair::<BaseElement>(), constraint.cc);

    // periodic value constraints should evaluate to trace_value - value
    let trace_value = BaseElement::rand();
    assert_eq!(
        trace_value - value,
        constraint.evaluate_at(BaseElement::rand(), trace_value)
    );

    // twiddle map was not touched
    assert!(twiddle_map.is_empty());
}

#[test]
fn boundary_constraint_from_sequence_assertion() {
    let mut test_prng = build_prng();
    let (inv_g, mut twiddle_map, mut prng) = build_constraint_params(16);

    // constraint should be built correctly for register 0, first step 0, stride 4
    let values = BaseElement::prng_vector([0; 32], 4);
    let constraint_poly = build_sequence_poly(&values, 16);
    let assertion = Assertion::sequence(0, 0, 4, values);
    let constraint = BoundaryConstraint::<BaseElement, BaseElement>::new(
        assertion,
        inv_g,
        &mut twiddle_map,
        &mut prng,
    );
    assert_eq!(0, constraint.register());
    assert_eq!(constraint_poly, constraint.poly());
    assert_eq!((0, BaseElement::ONE), constraint.poly_offset());
    assert_eq!(test_prng.draw_pair::<BaseElement>(), constraint.cc);
    assert_eq!(1, twiddle_map.len());

    // sequence value constraints with no offset should evaluate to
    // trace_value - constraint_poly(x)
    let x = BaseElement::rand();
    let trace_value = BaseElement::rand();
    assert_eq!(
        trace_value - polynom::eval(&constraint_poly, x),
        constraint.evaluate_at(x, trace_value)
    );

    // constraint should be built correctly for register 0, first step 3, stride 8
    let values = BaseElement::prng_vector([1; 32], 2);
    let constraint_poly = build_sequence_poly(&values, 16);
    let assertion = Assertion::sequence(0, 3, 8, values.clone());
    let constraint = BoundaryConstraint::<BaseElement, BaseElement>::new(
        assertion,
        inv_g,
        &mut twiddle_map,
        &mut prng,
    );
    assert_eq!(0, constraint.register());
    assert_eq!(constraint_poly, constraint.poly());
    assert_eq!((3, inv_g.exp(3)), constraint.poly_offset());
    assert_eq!(test_prng.draw_pair::<BaseElement>(), constraint.cc);
    assert_eq!(2, twiddle_map.len());

    // sequence value constraints with offset should evaluate to
    // trace_value - constraint_poly(x * offset)
    let x = BaseElement::rand();
    let trace_value = BaseElement::rand();
    assert_eq!(
        trace_value - polynom::eval(&constraint_poly, x * constraint.poly_offset().1),
        constraint.evaluate_at(x, trace_value)
    );
}

// HELPER FUNCTIONS
// ================================================================================================
fn build_constraint_params(
    trace_length: usize,
) -> (
    BaseElement,
    HashMap<usize, Vec<BaseElement>>,
    DefaultRandomElementGenerator<hash::Blake3_256>,
) {
    let inv_g = BaseElement::get_root_of_unity(log2(trace_length)).inv();
    let prng = build_prng();
    let twiddle_map = HashMap::<usize, Vec<BaseElement>>::new();
    (inv_g, twiddle_map, prng)
}
