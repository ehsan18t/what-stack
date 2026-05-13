//! Config-file based stack detection.

mod files;
mod python;
mod rules;

pub use rules::detect_from_config;
