// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use winter_crypto::hash;

pub fn blake3(c: &mut Criterion) {
    let v: [u8; 64] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
        19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
    ];
    let mut r = [0u8; 32];
    c.bench_function("hash_blake3", |bench| {
        bench.iter(|| hash::blake3(black_box(&v), black_box(&mut r)))
    });
}

pub fn sha3(c: &mut Criterion) {
    let v: [u8; 64] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
        19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
    ];
    let mut r = [0u8; 32];
    c.bench_function("hash_sha3", |bench| {
        bench.iter(|| hash::sha3(black_box(&v), black_box(&mut r)))
    });
}

criterion_group!(hash_group, blake3, sha3);
criterion_main!(hash_group);
