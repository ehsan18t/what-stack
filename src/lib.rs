//! Detect project roots and technology stacks from generic process metadata.
//!
//! `what-stack` is a small standalone library. It does not depend on Docker,
//! socket collection, async runtimes, logging, serialization, CLI parsing, or
//! any application-specific types. Callers pass ordinary strings and paths:
//! image names, project directories, process names, executable paths, working
//! directories, and command-line arguments.
//!
//! # Detection Model
//!
//! The crate exposes focused single-purpose functions plus [`StackDetector`]
//! for repeated lookups with caching:
//!
//! - [`detect_from_image`] parses container or artifact image names.
//! - [`detect_from_config`] scans one project-root directory for known project
//!   files.
//! - [`detect_from_process`] maps known runtime, server, database, and tool
//!   executable names.
//! - [`find_project_root`] walks upward from one directory until it finds a
//!   project marker.
//! - [`resolve_project_root`] applies the same project-root fallback order used
//!   by process collectors: current working directory, executable parent, then
//!   absolute command-line argument parents.
//! - [`StackDetector`] combines those rules and caches filesystem results.
//!
//! High-level stack detection intentionally uses this priority:
//!
//! 1. Image label.
//! 2. Project config label.
//! 3. Process label.
//!
//! Config labels are guarded in [`StackDetector`]: a project config is used only
//! when the process already looks like a known runtime, or when the executable
//! path belongs to the detected project root. This prevents unrelated helper
//! shells from inheriting a project's framework label just because their
//! current directory happens to be inside that project.
//!
//! # Scope
//!
//! This crate only detects labels. It does not discover running processes,
//! inspect network ports, query container engines, kill processes, read custom
//! rule files, or format user-facing output.
//!
//! # Examples
//!
//! Direct image and process detection:
//!
//! ```
//! use what_stack::{detect_from_image, detect_from_process};
//!
//! assert_eq!(
//!     detect_from_image("ghcr.io/org/nginx:latest").as_deref(),
//!     Some("Nginx"),
//! );
//! assert_eq!(detect_from_process("node.exe").as_deref(), Some("Node.js"));
//! ```
//!
//! Cached high-level detection:
//!
//! ```
//! use what_stack::{StackDetector, StackInput};
//!
//! let mut detector = StackDetector::default();
//! let label = detector.detect_stack(StackInput {
//!     image: Some("redis:7-alpine"),
//!     project_root: None,
//!     process_name: "",
//!     exe_name: None,
//!     exe_path: None,
//! });
//!
//! assert_eq!(label.as_deref(), Some("Redis"));
//! ```

mod config;
mod detector;
mod image;
mod process;
mod project;
mod types;

pub use config::detect_from_config;
pub use detector::StackDetector;
pub use image::detect_from_image;
pub use process::detect_from_process;
pub use project::{find_project_root, home_dir, project_name, resolve_project_root};
pub use types::{ProjectInput, StackInput, StackLabel};
