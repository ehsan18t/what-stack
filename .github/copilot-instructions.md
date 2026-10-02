# Copilot and AI Agent Instructions - what-stack

Treat every rule here as a hard constraint unless the human operator explicitly overrides it.

## Project Identity

| Field | Value |
| ----- | ----- |
| Language | Rust edition 2024 |
| Type | Library crate |
| Platform | Linux x86-64 and Windows x86-64 (macOS also tested in CI) |
| MSRV | Rust 1.88 (`rust-version` in Cargo.toml) |
| License | MIT |
| Repository | https://github.com/ehsan18t/what-stack |

## Module Map

| Path | Responsibility |
| ---- | -------------- |
| `src/lib.rs` | Crate docs, public re-exports, README doctests, cross-rule label consistency tests |
| `src/types.rs` | Public types: `StackKind`, `StackLabel`, `ProjectInput`, `StackInput` |
| `src/detector.rs` | `StackDetector`: cached project-root walks and config results, stack priority, config guard |
| `src/image.rs` | `detect_from_image`: exact and prefix image rules, companion-image filter, `dotnet` namespace |
| `src/process.rs` | `detect_from_process` and `detect_from_process_names`: process rules, version suffixes, truncated titles, `.exe` stripping |
| `src/project.rs` | Project markers, upward walk with home ceiling and depth cap, `resolve_project_root`, `home_dir` (libc on Unix) |
| `src/ecosystem.rs` | Internal ecosystem tags that limit which config labels a known runtime accepts |
| `src/config/mod.rs` | `detect_from_config` entry point |
| `src/config/rules.rs` | Ordered config rule tables and matching |
| `src/config/python.rs` | Python project and framework detection from entry and dependency files |
| `src/config/files.rs` | Directory listing and bounded, non-blocking, encoding-tolerant file reads |
| `tests/` | Integration, corpus, and property tests; see "Tests" in `docs/CONTRIBUTING.md` |
| `benches/benchmarks.rs` | Gungraun instruction-count benchmarks |

## Coding Rules

1. Clippy `all + pedantic + nursery` is denied, plus `unwrap_used` and `undocumented_unsafe_blocks`.
2. Do not use `unwrap()` outside tests.
3. Give every `unsafe` block a `// Safety:` comment.
4. Document every public item.
5. Keep functions under 100 lines and cognitive complexity under 30.
6. Do not commit `dbg!()`, `todo!()`, or `unimplemented!()`.
7. Do not add dependencies without maintainer approval.
8. Run `cargo fmt` before committing.
9. Use Conventional Commits: `<type>(<scope>): <description>`.
10. Update README or docs when behavior changes.
11. Keep tests independent of the host: fixtures for negative project walks live in a fake home `TempDir` that is passed as the home ceiling.

## Quality Gates

Run the quality gates listed in `docs/CONTRIBUTING.md` before pushing; that file is the only gate list. Run tests with `cargo test --lib --tests` and `cargo test --doc`, not `cargo test --all-targets`, which also runs the benchmark binary and needs `gungraun-runner`.
