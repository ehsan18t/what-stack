//! Config-file based stack detection.
//!
//! Config detection scans a single directory that the caller already considers
//! a project root. It does not walk parents or recurse into children.

mod files;
mod python;
mod rules;

pub use rules::detect_from_config;
