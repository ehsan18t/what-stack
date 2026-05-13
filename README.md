# what-stack

Universal Rust library for detecting project roots and technology stacks from
generic inputs such as image names, project directories, process names,
executable paths, and command-line arguments.

[![CI](https://github.com/ehsan18t/what-stack/actions/workflows/ci.yml/badge.svg)](https://github.com/ehsan18t/what-stack/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![docs.rs](https://docs.rs/what-stack/badge.svg)](https://docs.rs/what-stack)

## Goals

- Standalone library API with no PortLens, socket, Docker-client, or CLI types.
- Zero default runtime dependencies on Windows; Unix uses `libc` only for robust
  home-directory lookup.
- No async runtime, regex engine, logging facade, serde model, or subprocesses.
- Best-effort detection that prefers explicit evidence over port-number guesses.

## Install

```toml
[dependencies]
what-stack = "0.1"
```

## Quick Start

```rust
use std::path::Path;
use what_stack::{StackDetector, StackInput, detect_from_image, detect_from_process};

assert_eq!(detect_from_image("postgres:16").as_deref(), Some("PostgreSQL"));
assert_eq!(detect_from_process("NGINX.EXE").as_deref(), Some("Nginx"));

let mut detector = StackDetector::new(None);
let label = detector.detect_stack(StackInput {
    image: None,
    project_root: Some(Path::new(".")),
    process_name: "node",
    exe_name: None,
    exe_path: None,
});

println!("{label:?}");
```

## Detection Sources

`what-stack` detects stacks from:

- Image names, for example `postgres:16`, `redis/redis-stack`, or
  `mcr.microsoft.com/dotnet/aspnet`.
- Project config files, for example `next.config.mjs`, `Cargo.toml`, `go.mod`,
  `pyproject.toml`, `pom.xml`, `build.gradle`, `Gemfile`, and `.csproj`.
- Python entry and dependency files, including Django, Flask, FastAPI,
  Starlette, Litestar, and generic Python fallback.
- Process names, including common runtimes, databases, web servers, search
  services, message brokers, and dev tools.
- Project markers found by walking upward from cwd, executable parent, or
  absolute command-line argument paths.

High-level stack detection uses this priority:

1. Image name.
2. Project config, only when the process is recognized or the executable is
   inside the project.
3. Process name.

There is no well-known-port fallback.

## Development

Install Rust stable and the supported lint targets:

```bash
rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-msvc
```

Install local hooks:

```powershell
.\scripts\install-hooks.ps1
```

```bash
bash scripts/install-hooks.sh
```

## Quality Gates

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
instruction-count benchmarks.

```bash
cargo bench --bench benchmarks
```

## Contributing

See [CONTRIBUTING.md](docs/CONTRIBUTING.md).

## License

[MIT](LICENSE) - Copyright (c) 2026 Ehsan Khan
