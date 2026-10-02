#![allow(missing_docs, reason = "integration tests document behavior via names")]

//! Regression tests for project-walk and rule fixes after 0.1.0.

use std::borrow::Cow;
use std::path::Path;

use tempfile::TempDir;
use what_stack::{ProjectInput, StackDetector, StackInput, StackLabel, resolve_project_root};

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

/// Build a project from `(relative path, contents)` pairs.
fn project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    for (relative, contents) in files {
        write_file(dir.path(), relative, contents);
    }
    dir
}

fn node_label(root: &Path) -> Option<&'static str> {
    text(StackDetector::with_home(None).detect_stack(StackInput::new("node").project_root(root)))
}

#[test]
fn node_frameworks_from_package_dependencies() {
    for (files, expected) in [
        (
            &[(
                "package.json",
                r#"{"dependencies": {"next": "15.1.0", "react": "19"}}"#,
            )][..],
            "Next.js",
        ),
        (
            &[
                ("package.json", r#"{"dependencies": {"next": "15"}}"#),
                ("vite.config.ts", "export default {}"),
            ][..],
            "Next.js",
        ),
        (
            &[(
                "package.json",
                r#"{"dependencies": {"express": "^4.21.0"}}"#,
            )][..],
            "Express",
        ),
        (
            &[(
                "package.json",
                r#"{"dependencies": {"express": "4", "next": "15"}}"#,
            )][..],
            "Next.js",
        ),
        (
            &[
                ("package.json", r#"{"dependencies": {"express": "4"}}"#),
                ("vite.config.js", ""),
            ][..],
            "Vite",
        ),
        (
            &[("package.json", "{}"), ("nest-cli.json", "{}")][..],
            "NestJS",
        ),
        (
            &[(
                "package.json",
                r#"{"dependencies": {"@nestjs/core": "10", "express": "4"}}"#,
            )][..],
            "NestJS",
        ),
        (
            &[
                (
                    "package.json",
                    r#"{"devDependencies": {"@remix-run/dev": "2"}, "dependencies": {"@remix-run/react": "2"}}"#,
                ),
                ("vite.config.ts", ""),
            ][..],
            "Remix",
        ),
        (
            &[
                ("package.json", r#"{"dependencies": {"react-router": "7"}}"#),
                ("react-router.config.ts", ""),
                ("vite.config.ts", ""),
            ][..],
            "React Router",
        ),
        (
            &[
                (
                    "package.json",
                    r#"{"dependencies": {"@remix-run/router": "1", "react": "18"}}"#,
                ),
                ("vite.config.ts", ""),
            ][..],
            "Vite",
        ),
        (
            &[(
                "package.json",
                r#"{"name": "next", "scripts": {"next": "next dev"}, "peerDependencies": {"next": "15"}}"#,
            )][..],
            "Node.js",
        ),
    ] {
        let dir = project(files);
        assert_eq!(node_label(dir.path()), Some(expected), "{files:?}");
    }
}

#[test]
fn node_dependency_rules_stay_in_the_node_ecosystem() {
    let dir = project(&[
        ("package.json", r#"{"dependencies": {"express": "4"}}"#),
        ("go.mod", "module example.com/app\n"),
    ]);
    let mut detector = StackDetector::with_home(None);
    assert_eq!(
        text(detector.detect_stack(StackInput::new("go").project_root(dir.path()))),
        Some("Go")
    );
    assert_eq!(
        text(detector.detect_stack(StackInput::new("node").project_root(dir.path()))),
        Some("Express")
    );
    assert_eq!(config_text(dir.path()), Some("Go"));
}

#[test]
fn express_comes_after_every_non_node_rule() {
    // 0.1.0 had no Express rule, so these roots got the label of their
    // other build; a frontend or tooling `package.json` must not change that.
    const EXPRESS: (&str, &str) = ("package.json", r#"{"dependencies": {"express": "4"}}"#);
    for (other, expected) in [
        (
            (
                "Cargo.toml",
                "[package]
name = \"api\"
",
            ),
            "Rust",
        ),
        (
            (
                "go.mod",
                "module example.com/api
",
            ),
            "Go",
        ),
        (
            (
                "Web.csproj",
                "<Project />
",
            ),
            ".NET",
        ),
        (
            (
                "pom.xml",
                "<project />
",
            ),
            "Java (Maven)",
        ),
        (("composer.json", "{}"), "PHP"),
        (
            (
                "mix.exs",
                "defmodule Api.MixProject do
end
",
            ),
            "Elixir",
        ),
        (("deno.json", "{}"), "Deno"),
        (
            (
                "pyproject.toml",
                "[project]
name = \"api\"
",
            ),
            "Python",
        ),
    ] {
        let dir = project(&[EXPRESS, other]);
        assert_eq!(config_text(dir.path()), Some(expected), "{other:?}");
    }

    let dir = project(&[EXPRESS]);
    assert_eq!(config_text(dir.path()), Some("Express"));
}

#[test]
fn express_only_in_dev_dependencies_is_not_express() {
    // An npm library that starts an Express server in its tests.
    let dir = project(&[(
        "package.json",
        r#"{"name": "http-client", "devDependencies": {"express": "4", "supertest": "7"}}"#,
    )]);
    assert_eq!(config_text(dir.path()), None);
    assert_eq!(node_label(dir.path()), Some("Node.js"));
}

#[test]
fn symfony_projects_are_recognized_from_flex_lock_or_console() {
    let mut detector = StackDetector::with_home(None);
    for files in [
        &[("composer.json", "{}"), ("symfony.lock", "{}")][..],
        &[
            ("composer.json", "{}"),
            ("bin/console", "#!/usr/bin/env php\n"),
        ][..],
    ] {
        let dir = project(files);
        assert_eq!(config_text(dir.path()), Some("Symfony"), "{files:?}");
        assert_eq!(
            text(detector.detect_stack(StackInput::new("php-fpm").project_root(dir.path()))),
            Some("Symfony")
        );
    }

    let laravel = project(&[
        ("composer.json", "{}"),
        ("artisan", ""),
        ("symfony.lock", "{}"),
    ]);
    assert_eq!(config_text(laravel.path()), Some("Laravel"));

    let console_dir = project(&[("composer.json", "{}"), ("bin/console/readme", "")]);
    assert_eq!(
        config_text(console_dir.path()),
        Some("PHP"),
        "bin/console must be a file"
    );
}

const SPRING_POM: &str = r"<project>
  <parent>
    <groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.3.4</version>
  </parent>
</project>
";

#[test]
fn spring_boot_builds_are_recognized_for_jvm_processes() {
    let mut detector = StackDetector::with_home(None);
    for files in [
        &[("pom.xml", SPRING_POM)][..],
        &[(
            "build.gradle.kts",
            "plugins {\n    id(\"org.springframework.boot\") version \"3.3.4\"\n}\n",
        )][..],
        &[(
            "build.gradle",
            "dependencies {\n    implementation 'org.springframework.boot:spring-boot-starter-web'\n}\n",
        )][..],
    ] {
        let dir = project(files);
        assert_eq!(config_text(dir.path()), Some("Spring Boot"), "{files:?}");
        for process in ["java", "mvn", "gradle"] {
            assert_eq!(
                text(detector.detect_stack(StackInput::new(process).project_root(dir.path()))),
                Some("Spring Boot"),
                "{process} {files:?}"
            );
        }
    }

    for (files, expected) in [
        (
            &[(
                "pom.xml",
                "<project><!-- org.springframework.boot later -->\n<!--\n<dependency>\n  <groupId>org.springframework.boot</groupId>\n</dependency>\n-->\n</project>\n",
            )][..],
            "Java (Maven)",
        ),
        (
            &[(
                "build.gradle",
                "// id 'org.springframework.boot'\napply plugin: 'java'\n",
            )][..],
            "Java (Gradle)",
        ),
    ] {
        let dir = project(files);
        assert_eq!(config_text(dir.path()), Some(expected), "{files:?}");
    }
}

#[test]
fn gradle_settings_files_mark_multi_module_roots() {
    for (marker, expected) in [
        ("settings.gradle", "Java (Gradle)"),
        ("settings.gradle.kts", "Kotlin (Gradle)"),
    ] {
        let dir = project(&[(marker, "rootProject.name = \"shop\"\n")]);
        let nested = dir.path().join("docs/guide");
        std::fs::create_dir_all(&nested).expect("create nested dir");
        assert_eq!(
            what_stack::find_project_root(&nested, None).as_deref(),
            Some(dir.path()),
            "{marker}"
        );
        assert_eq!(config_text(dir.path()), Some(expected), "{marker}");
    }
}

#[test]
fn dotnet_solution_files_mark_project_roots() {
    for solution in ["Shop.sln", "Shop.slnx"] {
        let dir = project(&[
            (solution, ""),
            ("src/Shop.Api/Shop.Api.csproj", "<Project />"),
        ]);
        let tests_dir = dir.path().join("tests");
        std::fs::create_dir_all(&tests_dir).expect("create tests dir");
        assert_eq!(
            what_stack::find_project_root(&tests_dir, None).as_deref(),
            Some(dir.path()),
            "{solution}"
        );
        assert_eq!(config_text(dir.path()), Some(".NET"), "{solution}");
    }

    let both = project(&[("Shop.sln", ""), ("Shop.fsproj", "")]);
    assert_eq!(config_text(both.path()), Some(".NET (F#)"));
}

const PHOENIX_MIX: &str = r#"defmodule Shop.MixProject do
  use Mix.Project

  defp deps do
    [
      {:phoenix, "~> 1.7.14"},
      {:phoenix_live_view, "~> 1.0"}
    ]
  end
end
"#;

#[test]
fn phoenix_projects_and_windows_beam_processes() {
    let mut detector = StackDetector::with_home(None);
    let phoenix = project(&[("mix.exs", PHOENIX_MIX)]);
    assert_eq!(config_text(phoenix.path()), Some("Phoenix"));

    for process in ["erl.exe", "werl.exe", "beam.smp", "elixir"] {
        assert_eq!(
            text(detector.detect_stack(StackInput::new(process).project_root(phoenix.path()))),
            Some("Phoenix"),
            "{process}"
        );
    }
    assert_eq!(
        text(what_stack::detect_from_process("ERL.EXE")),
        Some("Erlang")
    );
    assert_eq!(
        text(what_stack::detect_from_process("werl")),
        Some("Erlang")
    );

    for mix in [
        "defp deps, do: [{:phoenix_pubsub, \"~> 2.1\"}]\n",
        "# {:phoenix, \"~> 1.7\"} once we add the web layer\ndefp deps, do: []\n",
    ] {
        let dir = project(&[("mix.exs", mix)]);
        assert_eq!(config_text(dir.path()), Some("Elixir"), "{mix}");
    }
}

#[test]
fn new_image_rules_and_connector_companions() {
    for (image, expected) in [
        ("confluentinc/cp-kafka:7.7.1", Some("Kafka")),
        (
            "mcr.microsoft.com/mssql/server:2022-latest",
            Some("SQL Server"),
        ),
        (
            "mcr.microsoft.com/mssql/rhel/server:2022-latest",
            Some("SQL Server"),
        ),
        ("pgvector/pgvector:pg17", Some("PostgreSQL")),
        ("amazoncorretto:21-alpine", Some("Java")),
        (
            "public.ecr.aws/amazoncorretto/amazoncorretto:17",
            Some("Java"),
        ),
        ("elixir:1.17-slim", Some("Elixir")),
        (
            "hexpm/elixir:1.17.3-erlang-27.1-alpine-3.20.3",
            Some("Elixir"),
        ),
        ("mcr.microsoft.com/dotnet/aspnet:8.0", Some(".NET")),
        ("confluentinc/cp-kafka-connect:7.7.1", None),
        ("debezium/kafka-connect:2.7", None),
        ("bitnami/kafka-connect", None),
        ("mcr.microsoft.com/mssql-tools", None),
        ("elixir-ls:latest", None),
        ("kafka:3.8", Some("Kafka")),
    ] {
        assert_eq!(
            text(what_stack::detect_from_image(image)),
            expected,
            "{image}"
        );
    }
}
