//! Property tests: detection never panics on arbitrary input, and the
//! documented invariances hold for every generated input, not just the
//! examples in the unit tests.
//!
//! Case counts are tuned so the whole file runs in a few seconds; the
//! filesystem properties use fewer cases than the pure string ones.

#![allow(missing_docs, reason = "integration tests document behavior via names")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use proptest::prelude::*;
use proptest::sample::select;
use tempfile::TempDir;
use what_stack::{
    ProjectInput, StackDetector, StackInput, detect_from_config, detect_from_image,
    detect_from_process, detect_from_process_names, find_project_root, resolve_project_root,
};

// ---------------------------------------------------------------------------
// Process names
// ---------------------------------------------------------------------------

/// Names the process rules know, including versioned and titled forms.
const KNOWN_PROCESSES: &[&str] = &[
    "node",
    "node20",
    "python3",
    "python3.12",
    "php-fpm8.2",
    "ruby3.2",
    "postgres",
    "redis-server",
    "next-server (v1",
    "gunicorn: maste",
    "puma 6.4.2 (tc",
    "beam.smp",
    "dotnet",
    "nginx",
    "w3wp",
    "sqlservr",
    "vite",
    "rails",
    "cargo",
    "java",
    "mvn",
    "deno",
    "bun",
];

/// Flip the ASCII case of each character where `mask` is set.
fn mix_case(text: &str, mask: &[bool]) -> String {
    text.chars()
        .zip(mask.iter().copied().chain(std::iter::repeat(false)))
        .map(|(c, upper)| {
            if upper {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

fn process_name() -> impl Strategy<Value = String> {
    prop_oneof![
        any::<String>(),
        "[a-zA-Z0-9._: ()-]{0,24}",
        select(KNOWN_PROCESSES).prop_map(String::from),
        (
            select(KNOWN_PROCESSES),
            "[0-9.]{0,6}",
            "[ :]?[a-z0-9 ()]{0,8}"
        )
            .prop_map(|(name, version, title)| format!("{name}{version}{title}")),
    ]
}

fn ends_with_exe(name: &str) -> bool {
    name.len() >= 4
        && name
            .get(name.len() - 4..)
            .is_some_and(|suffix| suffix.eq_ignore_ascii_case(".exe"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn process_detection_never_panics(name in process_name(), exe in proptest::option::of(process_name())) {
        drop(detect_from_process(&name));
        drop(detect_from_process_names(&name, exe.as_deref()));
    }

    #[test]
    fn process_detection_ignores_ascii_case(name in process_name(), mask in prop::collection::vec(any::<bool>(), 0..32)) {
        let expected = detect_from_process(&name);
        prop_assert_eq!(&detect_from_process(&name.to_ascii_uppercase()), &expected);
        prop_assert_eq!(&detect_from_process(&name.to_ascii_lowercase()), &expected);
        prop_assert_eq!(detect_from_process(&mix_case(&name, &mask)), expected);
    }

    #[test]
    fn process_detection_ignores_a_windows_exe_suffix(name in process_name(), suffix in select(&[".exe", ".EXE", ".Exe"][..])) {
        prop_assume!(!ends_with_exe(&name));
        prop_assert_eq!(detect_from_process(&format!("{name}{suffix}")), detect_from_process(&name));
    }

    #[test]
    fn known_process_names_match_in_any_case(name in select(KNOWN_PROCESSES), mask in prop::collection::vec(any::<bool>(), 0..32)) {
        let mixed = mix_case(name, &mask);
        let label = detect_from_process(&mixed);
        prop_assert!(label.is_some(), "{mixed:?} should be known");
        prop_assert_eq!(detect_from_process(&format!("{mixed}.exe")), label);
    }

    #[test]
    fn exe_name_is_only_a_fallback(name in process_name(), exe in process_name()) {
        let expected = detect_from_process(&name).or_else(|| detect_from_process(&exe));
        prop_assert_eq!(detect_from_process_names(&name, Some(&exe)), expected);
        prop_assert_eq!(detect_from_process_names(&name, None), detect_from_process(&name));
    }
}

// ---------------------------------------------------------------------------
// Image names
// ---------------------------------------------------------------------------

const KNOWN_IMAGES: &[&str] = &[
    "postgres",
    "postgresql",
    "postgis",
    "timescaledb-ha",
    "redis",
    "redis-stack-server",
    "nginx",
    "nginx-unprivileged",
    "node",
    "python",
    "mysql",
    "mariadb",
    "mongo",
    "traefik",
    "caddy",
    "openjdk",
    "eclipse-temurin",
    "golang",
    "php",
    "kafka",
    "rabbitmq",
    "postgres-exporter",
    "postgrest",
    "redis-commander",
    "redisinsight",
    "opensearch-dashboards",
];

fn image_base() -> impl Strategy<Value = String> {
    prop_oneof![
        select(KNOWN_IMAGES).prop_map(String::from),
        "[a-z0-9]([a-z0-9._-]{0,15}[a-z0-9])?",
    ]
}

/// Registry host (optionally with a port) plus namespaces, ending in `/`.
/// Never contains a `dotnet` segment, which is itself a detection rule.
fn image_prefix() -> impl Strategy<Value = String> {
    let registry = prop_oneof![
        Just(String::new()),
        Just(String::from("localhost:5000/")),
        Just(String::from("ghcr.io/")),
        Just(String::from("docker.io/library/")),
        Just(String::from("registry.example.com:443/")),
        "[a-z][a-z0-9-]{0,8}(\\.[a-z]{2,4})?(:[0-9]{2,5})?/",
    ];
    let namespaces = prop::collection::vec("[a-z0-9][a-z0-9_-]{0,9}", 0..3).prop_map(|parts| {
        parts
            .iter()
            .flat_map(|part| [part.as_str(), "/"])
            .collect::<String>()
    });

    (registry, namespaces)
        .prop_map(|(registry, namespaces)| format!("{registry}{namespaces}"))
        .prop_filter("a dotnet namespace is its own rule", |prefix| {
            !prefix
                .split(['/', ':'])
                .any(|segment| segment.eq_ignore_ascii_case("dotnet"))
        })
}

fn image_tag() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        Just(String::from(":latest")),
        Just(String::from(":16")),
        Just(String::from(":8.3-fpm-alpine")),
        ":[A-Za-z0-9_][A-Za-z0-9_.-]{0,20}",
    ]
}

fn image_digest() -> impl Strategy<Value = String> {
    prop_oneof![Just(String::new()), "@sha256:[0-9a-f]{64}"]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn image_detection_never_panics(image in any::<String>()) {
        drop(detect_from_image(&image));
    }

    #[test]
    fn image_detection_ignores_ascii_case(image in prop_oneof![any::<String>(), "[a-zA-Z0-9./:@_-]{0,40}"]) {
        let expected = detect_from_image(&image);
        prop_assert_eq!(&detect_from_image(&image.to_ascii_uppercase()), &expected);
        prop_assert_eq!(detect_from_image(&image.to_ascii_lowercase()), expected);
    }

    #[test]
    fn image_detection_ignores_registry_namespace_tag_and_digest(
        prefix in image_prefix(),
        base in image_base(),
        tag in image_tag(),
        digest in image_digest(),
    ) {
        let image = format!("{prefix}{base}{tag}{digest}");
        let expected = detect_from_image(&base);
        prop_assert_eq!(&detect_from_image(&image), &expected, "image {}", image);
        prop_assert_eq!(detect_from_image(&image.to_ascii_uppercase()), expected);
    }
}

// ---------------------------------------------------------------------------
// Config files
// ---------------------------------------------------------------------------

/// Files the config readers may open or list.
const CONFIG_FILES: &[&str] = &[
    "package.json",
    "pyproject.toml",
    "requirements.txt",
    "main.py",
    "app.py",
    "manage.py",
    "Pipfile",
    "uv.lock",
    "poetry.lock",
    "composer.json",
    "artisan",
    "pom.xml",
    "build.gradle.kts",
    "settings.gradle",
    "mix.exs",
    "Gemfile",
    "config.ru",
    "Cargo.toml",
    "go.mod",
    "deno.json",
    "next.config.js",
    "vite.config.ts",
];

/// Fragments that steer random contents toward what the readers look for.
const FRAGMENTS: &[&[u8]] = &[
    b"{",
    b"}",
    b"[",
    b"]",
    b"\"",
    b",",
    b"\n",
    b"\r\n",
    b"#",
    b"//",
    b"\"dependencies\": {",
    b"\"devDependencies\": {",
    b"\"next\": \"14.2.3\"",
    b"\"express\": \"^4\"",
    b"\"@nestjs/core\": \"^10\"",
    b"[project]\n",
    b"[tool.poetry.dependencies]\n",
    b"dependencies = [",
    b"\"flask>=3\"",
    b"django==5.0\n",
    b"from fastapi import FastAPI\n",
    b"app = FastAPI()\n",
    b"name = \"starlette\"\n",
    b"{:phoenix, \"~> 1.7\"}",
    b"org.springframework.boot",
    b"symfony/framework-bundle",
    b"\xFF\xFE",
    b"\xFE\xFF",
    b"\xEF\xBB\xBF",
    b"\xC3",
    b"\xE9",
    b"\0",
    b"\\u0000",
    b"\xF0\x9F\x92",
];

const BYTE_ORDER_MARKS: &[&[u8]] = &[b"\xFF\xFE", b"\xFE\xFF"];

fn file_contents() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        prop::collection::vec(any::<u8>(), 0..1024),
        prop::collection::vec(select(FRAGMENTS), 0..32).prop_map(|parts| parts.concat()),
        (
            select(BYTE_ORDER_MARKS),
            prop::collection::vec(any::<u8>(), 0..257)
        )
            .prop_map(|(bom, rest)| [bom, rest.as_slice()].concat()),
    ]
}

fn config_tree() -> impl Strategy<Value = BTreeMap<&'static str, Vec<u8>>> {
    prop::collection::btree_map(select(CONFIG_FILES), file_contents(), 1..6)
}

/// Processes covering every config ecosystem, plus an unknown one.
const CONFIG_PROCESSES: &[&str] = &[
    "node", "deno", "bun", "python3", "gunicorn", "ruby", "puma", "php", "java", "gradle",
    "dotnet", "cargo", "go", "beam.smp", "erl", "app",
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn config_readers_never_panic_on_arbitrary_file_bytes(files in config_tree()) {
        let dir = TempDir::new().expect("temp dir");
        for (name, contents) in &files {
            std::fs::write(dir.path().join(name), contents).expect("write config file");
        }

        let first = detect_from_config(dir.path());
        prop_assert_eq!(detect_from_config(dir.path()), first, "detection is deterministic");

        let exe = dir.path().join("app");
        let mut detector = StackDetector::with_home(None);
        for process in CONFIG_PROCESSES {
            drop(detector.detect_stack(
                StackInput::new(process)
                    .exe_path(exe.as_path())
                    .project_root(dir.path()),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Project root walk
// ---------------------------------------------------------------------------

const DIR_NAMES: &[&str] = &["a", "b", "src", "app", "node_modules", "Packages"];

/// Entry file names and whether each one is a project marker.
const ENTRIES: &[(&str, bool)] = &[
    ("package.json", true),
    ("Cargo.toml", true),
    ("go.mod", true),
    ("pyproject.toml", true),
    ("requirements.txt", true),
    ("Service.csproj", true),
    ("mix.exs", true),
    ("README.md", false),
    ("Cargo.toml.bak", false),
    ("notes.txt", false),
    ("index.js", false),
];

type DirPath = Vec<&'static str>;

fn dir_path(max_depth: usize) -> impl Strategy<Value = DirPath> {
    prop::collection::vec(select(DIR_NAMES), 0..=max_depth)
}

fn join(home: &Path, parts: &[&str]) -> PathBuf {
    parts
        .iter()
        .fold(home.to_path_buf(), |path, part| path.join(part))
}

/// Model of the walk: the nearest ancestor-or-self of `start`, strictly below
/// the home ceiling, that holds a marker entry.
fn expected_root(entries: &[(DirPath, (&str, bool))], start: &[&str]) -> Option<usize> {
    (1..=start.len()).rev().find(|&depth| {
        entries
            .iter()
            .any(|(dir, (_, marker))| *marker && dir.as_slice() == &start[..depth])
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn project_walk_finds_the_nearest_marker_below_home(
        entries in prop::collection::vec((dir_path(5), select(ENTRIES)), 0..12),
        starts in prop::collection::vec(dir_path(7), 1..6),
    ) {
        let home = TempDir::new().expect("fake home");
        for (dir, (name, _)) in &entries {
            let dir = join(home.path(), dir);
            std::fs::create_dir_all(&dir).expect("create dir");
            std::fs::write(dir.join(name), "").expect("write entry");
        }

        // One detector for every start, so cached answers are checked too.
        let mut detector = StackDetector::with_home(Some(home.path().to_path_buf()));
        for start in &starts {
            let start_path = join(home.path(), start);
            let expected = expected_root(&entries, start).map(|depth| join(home.path(), &start[..depth]));
            let exe = start_path.join("tool.exe");

            prop_assert_eq!(find_project_root(&start_path, Some(home.path())), expected.clone(), "start {:?}", start);
            prop_assert_eq!(
                resolve_project_root(ProjectInput::new().exe(exe.as_path()), Some(home.path())),
                expected.clone(),
                "exe parent of {:?}", start
            );
            prop_assert_eq!(
                detector.detect_project_root(ProjectInput::new().cwd(start_path.as_path())),
                expected,
                "cached start {:?}", start
            );
        }
    }
}
