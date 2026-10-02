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
//!   executable names; [`detect_from_process_names`] adds an executable-name
//!   fallback.
//! - [`find_project_root`] walks upward from one directory until it finds a
//!   project marker.
//! - [`resolve_project_root`] applies the project-root fallback order used by
//!   process collectors: current working directory, executable parent, then
//!   absolute command-line argument parents.
//! - [`StackDetector`] combines those rules and caches filesystem results.
//!
//! Every detected [`StackLabel`] carries a [`StackKind`] (runtime, framework,
//! tool, database, or service).
//!
//! High-level stack detection in [`StackDetector::detect_stack`] uses this
//! priority:
//!
//! 1. Image label.
//! 2. Process label, when it is final (framework, database, or service).
//! 3. Project config label.
//! 4. Process label (runtime or tool).
//!
//! Config labels are guarded: a project config is used only when the process
//! is a known runtime or tool, or when the process is unknown but its
//! executable path belongs to the project root. This keeps a `postgres` or
//! `nginx` process started from a Next.js folder labeled as itself, and keeps
//! unrelated helper shells from inheriting a project's framework label just
//! because their working directory happens to be inside that project.
//!
//! Config labels are also ecosystem-aware: a known runtime or tool accepts
//! only config labels from its own ecosystem. In a Laravel project with
//! `vite.config.js`, `php` is `Laravel` and `node` is `Vite`; a `python`
//! process in a Next.js folder stays `Python`.
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
//! use what_stack::{StackKind, detect_from_image, detect_from_process};
//!
//! let nginx = detect_from_image("ghcr.io/org/nginx:latest").expect("known image");
//! assert_eq!(nginx, "Nginx");
//! assert_eq!(nginx.kind(), StackKind::Service);
//! assert_eq!(detect_from_process("node.exe").expect("known process"), "Node.js");
//! ```
//!
//! Cached high-level detection:
//!
//! ```
//! use what_stack::{StackDetector, StackInput};
//!
//! let mut detector = StackDetector::new();
//! let label = detector.detect_stack(StackInput::new("").image("redis:7-alpine"));
//!
//! assert_eq!(label.expect("known image"), "Redis");
//! ```

mod config;
mod detector;
mod ecosystem;
mod image;
mod labels;
mod process;
mod project;
mod text;
mod types;

pub use config::detect_from_config;
pub use detector::StackDetector;
pub use image::detect_from_image;
pub use process::{detect_from_process, detect_from_process_names};
pub use project::{
    MAX_WALK_DEPTH, find_project_root, home_dir, project_name, resolve_project_root,
};
pub use types::{ProjectInput, StackInput, StackKind, StackLabel};

/// Compiles the README examples as doctests.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn every_label_text_has_exactly_one_kind_across_all_rules() {
        let mut kinds: HashMap<&str, StackKind> = HashMap::new();

        for label in labels::ALL {
            let previous = kinds.insert(label.as_str(), label.kind());
            assert!(
                previous.is_none_or(|kind| kind == label.kind()),
                "label {label} has conflicting kinds {previous:?} and {:?}",
                label.kind()
            );
        }

        assert!(
            kinds.len() > 40,
            "expected every built-in label to be scanned"
        );
    }

    #[test]
    fn stack_label_text_traits_ignore_kind_but_equality_does_not() {
        let runtime = StackLabel::from_static("Vite", StackKind::Runtime);
        let tool = StackLabel::new(String::from("Vite"), StackKind::Tool);

        assert_eq!(runtime, "Vite");
        assert_eq!("Vite", tool);
        assert_eq!(runtime.as_str(), tool.as_ref());
        assert_eq!(runtime.to_string(), "Vite");
        assert_ne!(runtime, tool);
        assert_eq!(String::from(tool.clone()), "Vite");
        assert_eq!(tool.into_cow(), "Vite");
    }

    #[test]
    fn builtin_labels_borrow_static_text() {
        for label in labels::ALL {
            assert!(
                matches!(label.clone().into_cow(), std::borrow::Cow::Borrowed(_)),
                "label {label} should not allocate"
            );
        }
    }
}
