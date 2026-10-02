//! Config-file based stack detection.
//!
//! Config detection scans a single directory that the caller already considers
//! a project root. It does not walk parents or recurse into children.

mod files;
mod node;
mod python;
mod rules;

use std::path::Path;

pub use rules::{ConfigScope, detect_for_scope, detect_from_config};

/// Whether `dir/Cargo.toml` declares a Cargo workspace (`[workspace]` or a
/// `[workspace.*]` table).
pub fn declares_cargo_workspace(dir: &Path) -> bool {
    files::read_text_file(&dir.join("Cargo.toml")).is_some_and(|text| {
        text.lines().map(str::trim_start).any(|line| {
            line.strip_prefix("[workspace")
                .is_some_and(|rest| rest.starts_with([']', '.']))
        })
    })
}
