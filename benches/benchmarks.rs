#![allow(
    missing_docs,
    reason = "Gungraun macros generate internal harness items"
)]

//! Benchmarks for `what-stack`.

use std::hint::black_box;

use gungraun::prelude::*;
use what_stack::greeting;

#[library_benchmark]
#[bench::short("world")]
#[bench::medium("benchmark input")]
#[bench::long("benchmark input with a longer string payload")]
fn bench_greeting(input: &str) -> usize {
    let output = greeting(black_box(input));
    black_box(output.len())
}

library_benchmark_group!(name = greeting_group, benchmarks = [bench_greeting]);
main!(library_benchmark_groups = [greeting_group]);
