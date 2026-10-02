#![allow(missing_docs, reason = "integration tests document behavior via names")]

//! Regression tests for project-walk and rule fixes after 0.1.0.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use what_stack::{
    MAX_WALK_DEPTH, ProjectInput, StackDetector, StackInput, StackLabel, resolve_project_root,
};

/// Label text for assertions. Built-in labels are static, so this also checks
/// that detection did not allocate.
fn text(label: Option<StackLabel>) -> Option<&'static str> {
    match label?.into_cow() {
        Cow::Borrowed(text) => Some(text),
        Cow::Owned(text) => panic!("built-in label {text:?} should be static"),
    }
}

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

fn detect_unknown(exe: &Path, project_root: &Path) -> Option<&'static str> {
    let label = StackDetector::with_home(None).detect_stack(
        StackInput::new("main")
            .exe_path(exe)
            .project_root(project_root),
    );
    text(label)
}

#[test]
fn go_run_binaries_get_the_go_label_from_the_project() {
    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "go.mod", "module example.com/app\n");
    write_file(project.path(), "package.json", "{}");
    write_file(project.path(), "next.config.js", "");

    let temp = TempDir::new().expect("temp dir");
    let go_run = temp.path().join("go-build2895466181/b001/exe/main");
    assert_eq!(detect_unknown(&go_run, project.path()), Some("Go"));

    let unrelated = temp.path().join("go-builder/b001/exe/main");
    assert_eq!(detect_unknown(&unrelated, project.path()), None);
}

#[test]
fn cargo_workspace_member_binaries_get_the_rust_label() {
    let workspace = TempDir::new().expect("temp dir");
    write_file(
        workspace.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"app\"]\n",
    );
    write_file(
        workspace.path(),
        "app/Cargo.toml",
        "[package]\nname = \"app\"\n",
    );
    let member = workspace.path().join("app");
    let exe = workspace.path().join("target/debug/app");
    assert_eq!(detect_unknown(&exe, &member), Some("Rust"));

    let release = workspace.path().join("target/release/app");
    assert_eq!(detect_unknown(&release, &member), Some("Rust"));

    let elsewhere = workspace.path().join("dist/app");
    assert_eq!(detect_unknown(&elsewhere, &member), None);

    let not_a_workspace = TempDir::new().expect("temp dir");
    write_file(
        not_a_workspace.path(),
        "Cargo.toml",
        "[package]\nname = \"outer\"\n",
    );
    write_file(
        not_a_workspace.path(),
        "app/Cargo.toml",
        "[package]\nname = \"app\"\n",
    );
    let exe = not_a_workspace.path().join("target/debug/app");
    assert_eq!(
        detect_unknown(&exe, &not_a_workspace.path().join("app")),
        None
    );
}

#[test]
fn unknown_binaries_inside_the_project_prefer_compiled_ecosystems() {
    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "go.mod", "module example.com/app\n");
    write_file(project.path(), "package.json", "{}");
    write_file(project.path(), "vite.config.js", "");
    let air_binary = project.path().join("tmp/main");
    assert_eq!(detect_unknown(&air_binary, project.path()), Some("Go"));

    let dotnet = TempDir::new().expect("temp dir");
    write_file(dotnet.path(), "Api.csproj", "<Project />");
    write_file(dotnet.path(), "package.json", "{}");
    write_file(dotnet.path(), "vite.config.ts", "");
    let app = dotnet.path().join("bin/Debug/net8.0/Api.exe");
    assert_eq!(detect_unknown(&app, dotnet.path()), Some(".NET"));

    // Without compiled config the usual order still applies.
    let web = TempDir::new().expect("temp dir");
    write_file(web.path(), "package.json", "{}");
    write_file(web.path(), "next.config.mjs", "");
    let helper = web.path().join("bin/helper");
    assert_eq!(detect_unknown(&helper, web.path()), Some("Next.js"));
    assert_eq!(
        text(what_stack::detect_from_config(project.path())),
        Some("Vite")
    );
}

fn config_text(root: &Path) -> Option<&'static str> {
    text(what_stack::detect_from_config(root))
}

#[test]
fn python_lock_files_do_not_add_transitive_frameworks() {
    let project = TempDir::new().expect("temp dir");
    write_file(
        project.path(),
        "pyproject.toml",
        "[project]\nname = \"agent\"\ndependencies = [\"mcp>=1.2\"]\n",
    );
    write_file(
        project.path(),
        "uv.lock",
        "[[package]]\nname = \"mcp\"\n\n[[package]]\nname = \"starlette\"\n",
    );
    assert_eq!(config_text(project.path()), Some("Python"));

    let poetry_only = TempDir::new().expect("temp dir");
    write_file(
        poetry_only.path(),
        "poetry.lock",
        "[[package]]\nname = \"fastapi\"\n",
    );
    assert_eq!(config_text(poetry_only.path()), Some("Python"));
}

#[test]
fn python_lock_files_confirm_declared_frameworks() {
    let project = TempDir::new().expect("temp dir");
    write_file(
        project.path(),
        "pyproject.toml",
        "[project]\ndescription = \"Port of our Flask app\"\ndependencies = [\"fastapi\"]\n",
    );
    write_file(
        project.path(),
        "uv.lock",
        "[[package]]\nname = \"fastapi\"\n\n[[package]]\nname = \"starlette\"\n",
    );
    assert_eq!(
        config_text(project.path()),
        Some("FastAPI"),
        "the lock rules out the Flask mention in the description"
    );

    let unlocked = TempDir::new().expect("temp dir");
    write_file(unlocked.path(), "requirements.txt", "fastapi==0.115\n");
    assert_eq!(config_text(unlocked.path()), Some("FastAPI"));

    // A lock longer than the read cap cannot rule a framework out.
    let large = TempDir::new().expect("temp dir");
    write_file(large.path(), "requirements.txt", "flask\n");
    let filler = "[[package]]\nname = \"other\"\n".repeat(4096);
    write_file(large.path(), "poetry.lock", &filler);
    assert_eq!(config_text(large.path()), Some("Flask"));
}

#[test]
fn python_comment_lines_do_not_declare_frameworks() {
    let project = TempDir::new().expect("temp dir");
    write_file(
        project.path(),
        "requirements.txt",
        "# flask was replaced by plain wsgi\n  # django too\nrequests==2.32\n",
    );
    assert_eq!(config_text(project.path()), Some("Python"));

    let pyproject = TempDir::new().expect("temp dir");
    write_file(
        pyproject.path(),
        "pyproject.toml",
        "[project]\n# TODO: move to fastapi\ndependencies = [\"litestar\"]\n",
    );
    assert_eq!(config_text(pyproject.path()), Some("Litestar"));
}
