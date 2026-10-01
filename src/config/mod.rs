//! Config-file based stack detection.
//!
//! Config detection scans a single directory that the caller already considers
//! a project root. It does not walk parents or recurse into children.

mod files;
mod python;
mod rules;

pub use rules::{detect_for_ecosystem, detect_from_config};

/// Every built-in label config detection can produce, for consistency tests.
#[cfg(test)]
pub fn all_labels() -> Vec<&'static crate::StackLabel> {
    static STANDALONE: [crate::StackLabel; 2] = [python::PYTHON_LABEL, python::DJANGO_LABEL];

    rules::CONFIG_RULES
        .iter()
        .chain(rules::LATE_CONFIG_RULES)
        .map(|(_, label, _)| label)
        .chain(STANDALONE.iter())
        .chain(
            python::PYTHON_SOURCE_PATTERNS
                .iter()
                .map(|(label, _, _)| label),
        )
        .chain(
            python::PYTHON_DEPENDENCY_PATTERNS
                .iter()
                .map(|(_, label)| label),
        )
        .collect()
}
