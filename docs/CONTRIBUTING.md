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
| 2 | `cargo clippy --locked --all-targets -- -D warnings` | Lints |
| 3 | `cargo test --locked --lib --tests && cargo test --locked --doc` | Tests |
| 4 | `cargo bench --locked --no-run` | Benchmarks compile |
| 5 | `cargo build --locked` | Build |
| 6 | `cargo doc --locked --no-deps` | Documentation |
| 7 | `cargo deny check` | Dependency audit |

To lint `cfg(unix)` and `cfg(windows)` code from a single machine, run `scripts/check-platform-clippy.sh` or `scripts/check-platform-clippy.ps1`, which runs Clippy for both the Linux and Windows targets installed above.

CI runs these gates on Linux, Windows, and macOS. It also runs two packaging checks:

| Check | Command | Purpose |
| ----- | ------- | ------- |
| MSRV | `cargo +1.88 check --locked --all-targets` | The declared `rust-version` (1.88) still compiles |
| Package | `cargo publish --locked --dry-run` | The crates.io package builds from the files listed in `include` |

Run `cargo package --list` to see exactly which files are published. Only `src/`, `LICENSE`, `README.md`, and `CHANGELOG.md` ship (plus the manifest and lockfile that Cargo adds); tests, benches, scripts, hooks, and tooling config stay in the repository.

## Changelog

Record user-visible changes under `## [Unreleased]` in `CHANGELOG.md` using the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) sections (`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`). On release, rename the section to the version and date and make sure the version in `Cargo.toml` matches; the release workflow fails if the tag and `Cargo.toml` disagree.

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
- The `[licenses] allow` list in `deny.toml` is the source of truth. It currently allows only MIT and Apache-2.0, the licenses the dependency graph uses; adding another license needs maintainer approval.
- `cargo deny check` must pass in CI.
- Do not add new dependencies without maintainer approval.
