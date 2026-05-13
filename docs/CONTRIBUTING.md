# Contributing to what-stack

## Development Setup

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

The hook installers resolve Git's actual hooks directory through Git metadata,
so they work in normal clones and linked worktrees.

## Quality Gates

| Gate | Command | Purpose |
| ---- | ------- | ------- |
| 1 | `cargo fmt --all -- --check` | Formatting |
| 2 | `scripts/check-platform-clippy.sh` / `scripts/check-platform-clippy.ps1` | Linux + Windows cfg linting |
| 3 | `cargo test --locked --lib --tests && cargo test --locked --doc` | Tests |
| 4 | `cargo bench --locked --no-run` | Benchmark harness compiles |
| 5 | `cargo build --locked` | Build |
| 6 | `cargo doc --locked --no-deps` | Documentation |
| 7 | `cargo deny check` | Dependency audit |


## Coding Standards

- Clippy `all + pedantic + nursery` is denied.
- Do not use `unwrap()` outside tests.
- Document every public item.
- Keep functions under 100 lines and cognitive complexity under 30.
- Do not commit `dbg!()`, `todo!()`, or `unimplemented!()`.

## Commit Messages

Use Conventional Commits:

```text
<type>(<scope>): <description>
```

Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`,
`build`, `ci`, `chore`, `revert`, and `enforce`.

Rules:

- Description starts lowercase.
- Description is 5-200 characters.
- No trailing period.
- Scope is optional and uses lowercase alphanumeric text plus hyphens.

## Benchmark Regression

Benchmarks use Gungraun instruction counts. Execution requires Valgrind and the
matching `gungraun-runner` package:

```bash
cargo install --version 0.18.2 gungraun-runner
cargo bench --bench benchmarks
```

To compare against a named baseline and fail on instruction regressions:

```bash
cargo bench --bench benchmarks -- --save-baseline main --callgrind-metrics=ir
cargo bench --bench benchmarks -- --baseline main --callgrind-metrics=ir --callgrind-limits='ir=1.0%'
```

## Dependency Policy

- Prefer `std` over external crates.
- Only MIT, Apache-2.0, BSD, MPL-2.0, Unicode-3.0, and Zlib-compatible crates.
- `cargo deny check` must pass in CI.
- Do not add new dependencies without maintainer approval.
