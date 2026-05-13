# Copilot and AI Agent Instructions - WhatStack

Treat every rule here as a hard constraint unless the human operator explicitly
overrides it.

## Project Identity

| Field | Value |
| ----- | ----- |
| Language | Rust edition 2024 |
| Type | Library crate |
| Platform | Linux x86-64 and Windows x86-64 |
| License | MIT |
| Repository | https://github.com/ehsan18t/what-stack |

## Coding Rules

1. Clippy `all + pedantic + nursery` is denied.
2. Do not use `unwrap()` outside tests.
3. Document every public item.
4. Keep functions under 100 lines and cognitive complexity under 30.
5. Do not commit `dbg!()`, `todo!()`, or `unimplemented!()`.
6. Do not add dependencies without maintainer approval.
7. Run `cargo fmt` before committing.
8. Use Conventional Commits: `<type>(<scope>): <description>`.
9. Update README or docs when behavior changes.

## Quality Gates

Run these before pushing:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --lib --tests
cargo test --locked --doc
cargo bench --locked --no-run
cargo build --locked
cargo doc --locked --no-deps
```
