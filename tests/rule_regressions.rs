#![allow(missing_docs, reason = "integration tests document behavior via names")]

//! Regression tests for project-walk and rule fixes after 0.1.0.

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use what_stack::{MAX_WALK_DEPTH, ProjectInput, StackDetector, resolve_project_root};

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

#[test]
fn installed_runtime_executable_does_not_make_its_install_tree_a_project() {
    let home = TempDir::new().expect("temp dir");
    write_file(home.path(), ".nvm/package.json", "{}");
    let bin = home.path().join(".nvm/versions/node/v20/bin");
    std::fs::create_dir_all(&bin).expect("create nvm bin dir");
    let node = bin.join("node");

    for home_ceiling in [Some(home.path()), None] {
        let input = ProjectInput::new().exe(node.as_path());
        assert_eq!(
            resolve_project_root(input, home_ceiling),
            None,
            "home ceiling {home_ceiling:?}"
        );
        let mut detector = StackDetector::with_home(home_ceiling.map(Path::to_path_buf));
        assert_eq!(detector.detect_project_root(input), None);
    }
}

#[test]
fn executable_roots_inside_home_dot_directories_are_rejected() {
    let home = TempDir::new().expect("temp dir");
    write_file(home.path(), ".local/share/tool/package.json", "{}");
    write_file(home.path(), "work/app/Cargo.toml", "");
    let installed = home.path().join(".local/share/tool/bin/tool");
    let built = home.path().join("work/app/target/debug/app");
    std::fs::create_dir_all(installed.parent().expect("parent")).expect("create dir");
    std::fs::create_dir_all(built.parent().expect("parent")).expect("create dir");

    let mut detector = StackDetector::with_home(Some(home.path().to_path_buf()));
    let from_installed = ProjectInput::new().exe(installed.as_path());
    assert_eq!(
        resolve_project_root(from_installed, Some(home.path())),
        None
    );
    assert_eq!(detector.detect_project_root(from_installed), None);

    let from_built = ProjectInput::new().exe(built.as_path());
    let expected = home.path().join("work/app");
    assert_eq!(
        resolve_project_root(from_built, Some(home.path())).as_deref(),
        Some(expected.as_path())
    );
    assert_eq!(
        detector.detect_project_root(from_built).as_deref(),
        Some(expected.as_path())
    );

    // Without a home ceiling there is no dot-directory rule.
    assert_eq!(
        resolve_project_root(from_installed, None).as_deref(),
        Some(home.path().join(".local/share/tool").as_path())
    );
}
