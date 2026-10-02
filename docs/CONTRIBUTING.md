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

The hook installers resolve Git's actual hooks directory through Git metadata, so they work in normal clones and linked worktrees. They install the POSIX hooks from `hooks/`, which Git for Windows also runs (through its bundled `sh`). The `.bat` files in `hooks/` are equivalents for setups that run batch hooks. The pre-commit and pre-push hooks fall back to `~/.cargo/bin` when Git runs them with a `PATH` that lacks `cargo`.

## Quality Gates

This is the single list of gates. CI, the hooks, and the README refer to it.

| Gate | Command | Purpose |
| ---- | ------- | ------- |
| 1 | `cargo fmt --all -- --check` | Formatting |
| 2 | `cargo clippy --locked --all-targets -- -D warnings` | Lints |
| 3 | `cargo test --locked --lib --tests && cargo test --locked --doc` | Tests |
| 4 | `cargo bench --locked --no-run` | Benchmarks compile |
| 5 | `cargo build --locked` | Build |
| 6 | `cargo doc --locked --no-deps` | Documentation |
| 7 | `cargo deny check` | Dependency audit |

The pre-commit hook runs gates 1 to 3; the pre-push hook runs all seven, and treats a local `cargo deny` failure as a warning because CI enforces it.

To lint `cfg(unix)` and `cfg(windows)` code from a single machine, run `scripts/check-platform-clippy.sh` or `scripts/check-platform-clippy.ps1`, which runs Clippy for both the Linux and Windows targets installed above. The hooks use these scripts for gate 2.

Gate 3 deliberately names `--lib --tests` and `--doc` instead of `--all-targets`. `cargo test --all-targets` also runs the Gungraun benchmark binary in test mode, which fails unless `gungraun-runner` is installed (see [Benchmark Regression](#benchmark-regression)). Use `--all-targets` only for `cargo clippy` and `cargo check`.

CI runs these gates on Linux, Windows, and macOS. It also runs two packaging checks:

| Check | Command | Purpose |
| ----- | ------- | ------- |
| MSRV | `cargo +1.88 check --locked --all-targets` | The declared `rust-version` (1.88) still compiles |
| Package | `cargo publish --locked --dry-run` | The crates.io package builds from the files listed in `include` |

Run `cargo package --list` to see exactly which files are published. Only `src/`, `LICENSE`, `README.md`, and `CHANGELOG.md` ship (plus the manifest and lockfile that Cargo adds); tests, benches, scripts, hooks, and tooling config stay in the repository.

## Tests

| Location | What it covers |
| -------- | -------------- |
| `src/**` `#[cfg(test)]` modules | Private helpers: path comparison, home lookup, text decoding, caches |
| `tests/stack_detection.rs` | Public API behavior, one rule or guard per test |
| `tests/corpus.rs` | Realistic project trees run through root resolution and stack detection, one test per fixture |
| `tests/proptest_detection.rs` | Property tests: no panics on arbitrary input, case and suffix invariance, a model of the project walk |

Tests must not depend on the machine they run on. A test that expects an upward walk to find nothing creates its fixtures inside a fake home `TempDir` and passes it as the home ceiling (`StackDetector::with_home` or the `home` argument), because a stray `package.json` above the system temp directory would otherwise be found. On Windows `%TEMP%` lives under the user profile, where such files are common.

Corpus cases for behavior that is agreed but not implemented yet carry `#[ignore = "pending rule: ..."]`. Run them with `cargo test --test corpus -- --ignored`, and remove the attribute in the change that implements the rule.

If a property test fails, proptest writes the minimal failing input to a `*.proptest-regressions` file next to the test. Commit that file with the fix so the case is replayed on every run.

## Changelog

Record user-visible changes under `## [Unreleased]` in `CHANGELOG.md` using the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) sections (`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`). On release, rename the section to the version and date and make sure the version in `Cargo.toml` matches; the release workflow fails if the tag and `Cargo.toml` disagree.

## Coding Standards

- Clippy `all + pedantic + nursery` is denied.
- Do not use `unwrap()` outside tests; Clippy's `unwrap_used` enforces this.
- Every `unsafe` block carries a `// Safety:` comment; Clippy's `undocumented_unsafe_blocks` enforces this.
- Document every public item.
- Keep functions under 100 lines and cognitive complexity under 30.
- Do not commit `dbg!()`, `todo!()`, or `unimplemented!()`.

## Commit Messages

Use Conventional Commits:

```text
<type>(<scope>): <description>
```

Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`, and `enforce`.

Rules:

- Description starts lowercase.
- Description is 5-200 characters.
- No trailing period.
- Scope is optional and uses lowercase alphanumeric text plus hyphens.

## Benchmark Regression

Benchmarks use Gungraun instruction counts. Execution requires Valgrind and the `gungraun-runner` version that matches the `gungraun` dev-dependency:

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
- Library dependencies declare the oldest version the crate works with (`libc = "0.2"`), so consumers are not forced to upgrade. Dev-dependencies may pin exact minimums because they never reach consumers.
- The `[licenses] allow` list in `deny.toml` is the source of truth. It currently allows only MIT and Apache-2.0, the licenses the dependency graph uses; adding another license needs maintainer approval.
- `cargo deny check` must pass in CI.
- Do not add new dependencies without maintainer approval.
