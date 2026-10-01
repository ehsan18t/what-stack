#![allow(
    missing_docs,
    reason = "Gungraun macros generate internal harness items"
)]

//! Benchmarks for `what-stack`.

use std::hint::black_box;

use gungraun::prelude::*;
use what_stack::{detect_from_image, detect_from_process, detect_from_process_names};

#[library_benchmark]
#[bench::postgres("postgres:16")]
#[bench::dotnet("mcr.microsoft.com/dotnet/aspnet:8.0")]
#[bench::miss("prom/node-exporter:latest")]
fn bench_detect_from_image(image: &str) -> usize {
    let label = detect_from_image(black_box(image));
    black_box(label.as_ref().map_or(0, |label| label.as_str().len()))
}

#[library_benchmark]
#[bench::node("node")]
#[bench::windows_nginx("NGINX.EXE")]
#[bench::miss("node-exporter")]
fn bench_detect_from_process(process: &str) -> usize {
    let label = detect_from_process(black_box(process));
    black_box(label.as_ref().map_or(0, |label| label.as_str().len()))
}

#[library_benchmark]
#[bench::process_hit("node", Some("node.exe"))]
#[bench::exe_fallback("redis-serv", Some("redis-server"))]
#[bench::miss("helper", None)]
fn bench_detect_from_process_names(process: &str, exe: Option<&str>) -> usize {
    let label = detect_from_process_names(black_box(process), black_box(exe));
    black_box(label.as_ref().map_or(0, |label| label.as_str().len()))
}

library_benchmark_group!(
    name = detection_group,
    benchmarks = [
        bench_detect_from_image,
        bench_detect_from_process,
        bench_detect_from_process_names
    ]
);
main!(library_benchmark_groups = [detection_group]);
