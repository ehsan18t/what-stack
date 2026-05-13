//! Universal Rust library for detecting project roots and technology stacks.
//!
//! `what-stack` is intentionally standalone: it accepts generic strings and
//! filesystem paths instead of depending on process, socket, Docker, or CLI
//! types from a specific application.

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
