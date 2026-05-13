# what-stack

Universal Rust library for detecting project roots and technology stacks

[![CI](https://github.com/ehsan18t/what-stack/actions/workflows/ci.yml/badge.svg)](https://github.com/ehsan18t/what-stack/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![docs.rs](https://docs.rs/what-stack/badge.svg)](https://docs.rs/what-stack)


## Getting Started

```toml
[dependencies]
what-stack = "0.1"
```


## Development

### Prerequisites

- Rust stable toolchain
- Supported lint targets:

```bash
rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-msvc
```

- Optional dependency audit: `cargo install cargo-deny`
- Benchmark execution: Valgrind and `cargo install --version 0.18.2 gungraun-runner`


### Install Git Hooks

```powershell
.\scripts\install-hooks.ps1
```

```bash
bash scripts/install-hooks.sh
```

### Quality Gates

| Gate | Command | Purpose |
| ---- | ------- | ------- |
| 1 | `cargo fmt --all -- --check` | Formatting |
| 2 | `cargo clippy --locked --all-targets -- -D warnings` | Lints |
| 3 | `cargo test --locked --lib --tests && cargo test --locked --doc` | Tests |
| 4 | `cargo bench --locked --no-run` | Benchmarks compile |
| 5 | `cargo build --locked` | Build |
| 6 | `cargo doc --locked --no-deps` | Documentation |
| 7 | `cargo deny check` | Dependency audit |


## Benchmarks

This crate uses [Gungraun](https://crates.io/crates/gungraun) for deterministic
instruction-count benchmarks. Benchmark execution requires `gungraun-runner`
and Valgrind on a supported host. CI compiles benchmarks on Linux and Windows,
and runs regression checks on the hosted Ubuntu runner where Valgrind setup is
available.

```bash
cargo bench --bench benchmarks
```

To compare against a baseline:

```bash
cargo bench --bench benchmarks -- --save-baseline main --callgrind-metrics=ir
cargo bench --bench benchmarks -- --baseline main --callgrind-metrics=ir --callgrind-limits='ir=1.0%'
```

## Contributing

See [CONTRIBUTING.md](docs/CONTRIBUTING.md).

## License

[MIT](LICENSE) - Copyright (c) 2026 Ehsan Khan
