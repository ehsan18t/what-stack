#![allow(missing_docs, reason = "integration tests document behavior via names")]

//! Regression tests for project-walk and rule fixes after 0.1.0.

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use what_stack::{MAX_WALK_DEPTH, ProjectInput, StackDetector};

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("test path has parent"))
        .expect("create parent directory");
    std::fs::write(path, contents).expect("write test file");
}

#[test]
fn depth_capped_miss_does_not_hide_the_root_from_shallower_starts() {
    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "package.json", "{}");

    let chain: Vec<PathBuf> = (0..MAX_WALK_DEPTH + 6)
        .scan(project.path().to_path_buf(), |dir, index| {
            dir.push(format!("d{index}"));
            Some(dir.clone())
        })
        .collect();
    let deepest = chain.last().expect("chain is not empty");
    std::fs::create_dir_all(deepest).expect("create deep dir");

    let mut detector = StackDetector::with_home(None);
    assert_eq!(
        detector.detect_project_root(ProjectInput::new().cwd(deepest.as_path())),
        None,
        "the root is beyond MAX_WALK_DEPTH from the deepest directory"
    );

    // The deepest walk visited this directory, but the root is only eight
    // directories above it.
    let shallower = &chain[7];
    assert_eq!(
        detector
            .detect_project_root(ProjectInput::new().cwd(shallower.as_path()))
            .as_deref(),
        Some(project.path())
    );
}
