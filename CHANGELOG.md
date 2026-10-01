# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/ehsan18t/what-stack/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ehsan18t/what-stack/releases/tag/v0.1.0
