# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-10-02

### Added

- Node framework labels from `package.json` dependencies, read with a small built-in scanner (no JSON dependency): `Next.js` from `next` without a config file, `NestJS` from `@nestjs/core` or `nest-cli.json`, `Remix` from Remix packages such as `@remix-run/react` (ahead of `vite.config.*`), and `Express` from `express` under `dependencies` (not `devDependencies` alone, which an npm library with an Express test server has). Express is checked after every other rule, Python detection included, so a repo whose `package.json` lists `express` beside `Cargo.toml`, `go.mod`, or a `.csproj` keeps the `Rust`, `Go`, or `.NET` label it got from 0.1.0. Only the top-level `dependencies` and `devDependencies` maps are read: the same keys nested under `pnpm.packageExtensions` or `overrides`, or inside strings, do not count. `react-router.config.*` is `React Router` (React Router v7 framework mode), ahead of `vite.config.*`.
- `Symfony` for PHP projects with `composer.json` plus `symfony.lock` or `bin/console`.
- `Spring Boot` for Maven and Gradle builds that use `org.springframework.boot` (the parent POM, a starter dependency, or the Gradle plugin), outside comment lines. `java`, `mvn`, and `gradle` processes in such a project now get this label.
- `settings.gradle` and `settings.gradle.kts` are project-root markers and Gradle config rules, so a multi-module Gradle root without its own build script is found and labeled `Java (Gradle)` or `Kotlin (Gradle)`.
- `.sln` and `.slnx` solution files are project-root markers and `.NET` config rules, after `.csproj` and `.fsproj`, so a solution root whose projects live in subdirectories is found and labeled.
- `Phoenix` for Elixir projects whose `mix.exs` depends on `:phoenix` (not `:phoenix_pubsub` or other `:phoenix_*` packages alone), outside comment lines.
- `erl` and `werl` process names (the BEAM on Windows) map to `Erlang`, in the Erlang and Elixir ecosystem, so they pick up `Elixir` and `Phoenix` project config.
- Images: `confluentinc/cp-kafka` is `Kafka`, `mcr.microsoft.com/mssql/server` (any image under an `mssql` namespace) is `SQL Server`, `pgvector/pgvector` is `PostgreSQL`, `amazoncorretto` is `Java`, and `elixir` is `Elixir`.

### Changed

- `connect` is a companion image segment, so `kafka-connect`, `cp-kafka-connect`, and similar connector workers no longer get the `Kafka` label.
- `remix.config.*` is checked before `vite.config.*`, so a project with both is `Remix` instead of `Vite`.
- `svelte.config.*` is `SvelteKit` only when `package.json` lists `@sveltejs/kit` in `dependencies` or `devDependencies`. A plain Svelte app built with Vite, which also has `svelte.config.js`, is now `Vite` instead of `SvelteKit`.
- The `libc` dependency (Unix only) now accepts any `0.2` release instead of requiring `0.2.186` or newer, so the crate no longer forces a libc upgrade on its users.
- Reading a project file on Windows skips a redundant metadata call before opening it, which roughly halves the cost of each config file read.

### Fixed

- `find_project_root` and the other project walks test the current directory for a relative single-name start: `find_project_root("src")` with `Cargo.toml` in the working directory returns `.` instead of `None`.
- The executable-parent fallback of `resolve_project_root` and `StackDetector::detect_project_root` is skipped for known runtimes and tools, and a root it finds inside a dot directory directly under the home ceiling is rejected. An nvm `node` at `~/.nvm/versions/node/v20/bin/node` next to `~/.nvm/package.json` no longer makes `~/.nvm` the project root. A runtime inside a Python virtual environment still finds the project around it: with `pyvenv.cfg` in `app/.venv`, a `python` at `app/.venv/bin/python` (`app\.venv\Scripts\python.exe` on Windows) walks from the environment directory and reaches `app`, as in 0.1.0, and an environment created in place (`python -m venv .` inside the project) finds that project directly, while a conda environment (no `pyvenv.cfg`) is skipped and one under a home dot directory such as `~/.virtualenvs` is rejected.
- `StackDetector::detect_stack` uses project config for an unknown process built by `go run` or `go test` (its executable is under a temporary `go-build<digits>` directory; a plain `go-build` directory does not count), limited to Go config, and for a Cargo workspace member's binary under `<workspace>/target/` when the project root is the member crate, limited to Rust config. Both previously got no label.
- An unknown process whose executable is inside the project root now tries Rust, Go, .NET, and JVM config before the other rules. A Go binary rebuilt by air at `./tmp/main` in a repo with `go.mod`, `package.json`, and `vite.config.js` is `Go` instead of `Vite`. An executable under the project's `node_modules`, such as the native binary of esbuild, turbo, Biome, or SWC, tries Node config first instead, so esbuild in that repo is still `Vite`.
- Python framework detection no longer reads frameworks from lock files or comment lines. `uv.lock` and `poetry.lock` list transitive dependencies, so a project depending on `mcp` was labeled `Starlette`; lock files now only confirm a framework that `pyproject.toml`, `requirements*.txt`, `Pipfile`, or `setup.py` names, and `#` comment lines in those files are skipped.
- `StackDetector::detect_project_root` no longer caches misses from a walk that stopped at `MAX_WALK_DEPTH`. Such a miss was reused for shallower directories on the same path, which returned `None` even when their own walk would reach the project root.

## [0.1.0] - 2026-10-01

Initial release. `what-stack` is a dependency-light library that detects project roots and technology stacks from generic inputs, with no async runtime, regex engine, logging facade, or subprocesses.

### Added

- `StackLabel`, a label with display text and a `StackKind` (`Runtime`, `Framework`, `Tool`, `Database`, `Service`; `#[non_exhaustive]`). Built-in labels are `&'static str` and never allocate; `StackLabel::new` and `StackLabel::from_static` build custom ones.
- Builder inputs `StackInput` (`new(process_name)`, `image`, `project_root`, `exe_name`, `exe_path`) and `ProjectInput` (`new`, `cwd`, `exe`, `cmd`). Setters accept a value or an `Option`.
- `StackDetector` with `new`, `with_home`, `home`, `clear`, `detect_stack`, and `detect_project_root`. It caches project-root walks and per-ecosystem config results until `clear` is called, and stops upward walks before the home directory.
- Pure functions: `detect_from_process`, `detect_from_process_names` (process name with executable-name fallback), `detect_from_image`, `detect_from_config`, `find_project_root`, `resolve_project_root`, `project_name`, and `home_dir` (sudo-aware on Unix).
- `MAX_WALK_DEPTH` (64), the bound on directories tested per upward walk.
- Detection priority in `detect_stack`: image name, then a final process label (framework, database, service), then project config for runtime and tool processes (or unknown processes whose executable is inside the project), then the process label. There is no well-known-port fallback.
- Ecosystem-aware config: a known process accepts only config labels from its own ecosystem, so `php` in a Laravel project with `vite.config.js` is `Laravel`, `node` there is `Vite`, `node` next to `deno.json` stays `Node.js`, and `gunicorn` in a plain Python project stays `Gunicorn`.
- Safe file reads: Python entry and dependency files are read only when they are regular files (a FIFO or device under a scanned name never blocks), capped at 64 KiB, and decoded tolerantly (UTF-16 with a byte-order mark, lossy UTF-8).
- Image matching on the base name without registry, tag, or digest. Most runtime images match exactly; databases, services, and the `openjdk`, `eclipse-temurin`, and `dotnet` images match a prefix plus separator. Companion images (`postgres-exporter`, `opensearch-dashboards`, `redis-commander`, `mysql-workbench`, `traefik-forward-auth`) get no label.
- Process matching is exact and case-insensitive, ignores `.exe`, strips runtime version suffixes (`python3.12`, `php-fpm8.2`), and reads Linux-truncated titles (`next-server (v1`, `puma 6.4.2 (tc`, `gunicorn: maste`).
- Supported sources: process names for common runtimes, app servers (Gunicorn, Uvicorn, Puma), databases, web servers, search engines, message brokers (RabbitMQ, Kafka), and dev tools; images for the same databases and services plus PostGIS, TimescaleDB, and language runtimes; config files for Next.js, Nuxt, Angular, SvelteKit, Astro, Vite, Remix, Gatsby, Vue CLI, Webpack, Rust, Go, Maven, Gradle, Laravel, PHP, Elixir, Deno, Rails, Rack, .NET, and Python frameworks (Django, Flask, FastAPI, Starlette, Litestar); project markers such as `package.json`, `Cargo.toml`, `go.mod`, `pyproject.toml`, `composer.json`, `Gemfile`, `mix.exs`, and `.csproj`.
- Linux, Windows, and macOS support. Minimum supported Rust version: 1.88.

[Unreleased]: https://github.com/ehsan18t/what-stack/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/ehsan18t/what-stack/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ehsan18t/what-stack/releases/tag/v0.1.0
