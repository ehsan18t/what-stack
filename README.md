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
use what_stack::{
    ProjectInput, StackDetector, StackInput, StackKind, detect_from_image, detect_from_process,
};

// One-off detection from strings. Every label carries a kind.
let postgres = detect_from_image("postgres:16").expect("known image");
assert_eq!(postgres, "PostgreSQL");
assert_eq!(postgres.kind(), StackKind::Database);
assert_eq!(detect_from_process("NGINX.EXE").expect("known process"), "Nginx");

// Cached detection for a scan of many processes. The home ceiling defaults
// to `what_stack::home_dir()`; use `StackDetector::with_home` to override it.
let mut detector = StackDetector::new();

let root = detector.detect_project_root(ProjectInput::new().cwd(Path::new(".")));
let label = detector.detect_stack(
    StackInput::new("node")
        .exe_name("node.exe")
        .project_root(root.as_deref()),
);

if let Some(label) = label {
    println!("{label} ({:?})", label.kind());
}

// Drop cached filesystem results before the next scan.
detector.clear();
```

## API

| Item | Purpose |
| ---- | ------- |
| `StackLabel` | Label text plus `StackKind`. `as_str()`, `kind()`, `Display`, `AsRef<str>`, `== "text"`, `into_cow()`. |
| `StackKind` | `Runtime`, `Framework`, `Tool`, `Database`, `Service` (non-exhaustive). |
| `StackDetector` | Cached detection. `new()` / `Default` (home ceiling from `home_dir()`), `with_home(Option<PathBuf>)`, `detect_project_root`, `detect_stack`, `clear`, `home`. |
| `StackInput` | `StackInput::new(process_name)` with `.image()`, `.project_root()`, `.exe_name()`, `.exe_path()`. |
| `ProjectInput` | `ProjectInput::new()` with `.cwd()`, `.exe()`, `.cmd()`. |
| `detect_from_image` | Container or artifact image name. |
| `detect_from_process` | Process executable name. |
| `detect_from_process_names` | Process name with an executable-name fallback. |
| `detect_from_config` | Config files in one project-root directory. |
| `find_project_root` | Upward marker walk from one directory, with an optional home ceiling. |
| `resolve_project_root` | Uncached cwd, executable, and argument fallback walk. |
| `project_name` | Display name from a project-root path. |
| `home_dir` | Current user's home directory (sudo-aware on Unix). |
| `MAX_WALK_DEPTH` | Maximum directories tested per upward walk. |

Setters accept either a value or an `Option`, so `.exe_path(path)` and `.exe_path(maybe_path)` both work.

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
