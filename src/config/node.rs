//! Dependency names from `package.json` without a JSON parser.
//!
//! Only the keys of the top-level `"dependencies"` and `"devDependencies"`
//! objects are needed, and those objects are flat maps from package name to
//! version string. A small scanner over the (capped, already decoded) text finds them
//! without adding a JSON dependency to the crate.

/// Package names from the dependency maps of a `package.json`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct NodeDependencies {
    /// Keys of `"dependencies"`: packages the program needs at run time.
    pub runtime: Vec<String>,
    /// Keys of `"devDependencies"`: build, test, and tooling packages.
    pub dev: Vec<String>,
}

impl NodeDependencies {
    /// Whether any of `packages` is a runtime dependency.
    pub fn has_runtime(&self, packages: &[&str]) -> bool {
        contains_any(&self.runtime, packages)
    }

    /// Whether any of `packages` is a runtime or development dependency.
    pub fn has_any(&self, packages: &[&str]) -> bool {
        self.has_runtime(packages) || contains_any(&self.dev, packages)
    }
}

fn contains_any(names: &[String], packages: &[&str]) -> bool {
    names.iter().any(|name| packages.contains(&name.as_str()))
}

/// Package names listed under the top-level `"dependencies"` and
/// `"devDependencies"` keys.
///
/// The scanner walks the top-level object key by key and skips every other
/// value whole, so the same keys nested deeper (inside `"pnpm"`,
/// `"overrides"`, or `"workspaces"`) or inside strings do not count. A leading
/// byte-order mark is ignored. Malformed or truncated input yields the names
/// read before the problem.
pub fn dependency_names(json: &str) -> NodeDependencies {
    let mut dependencies = NodeDependencies::default();
    scan_top_level(json, &mut dependencies);
    dependencies
}

/// Read the top-level object of `json` into `dependencies`, stopping at the
/// first malformed or truncated part.
fn scan_top_level(json: &str, dependencies: &mut NodeDependencies) -> Option<()> {
    let mut rest = json
        .trim_start_matches(|c: char| c == '\u{feff}' || c.is_whitespace())
        .strip_prefix('{')?;

    loop {
        rest = rest.trim_start();
        if rest.starts_with('}') {
            return Some(());
        }
        let (key, after_key) = rest.strip_prefix('"').and_then(split_json_string)?;
        let value = after_key.trim_start().strip_prefix(':')?.trim_start();
        let names = match key {
            "dependencies" => Some(&mut dependencies.runtime),
            "devDependencies" => Some(&mut dependencies.dev),
            _ => None,
        };
        let after_value = match (names, value.strip_prefix('{')) {
            (Some(names), Some(object)) => collect_object_keys(object, names)?,
            _ => skip_value(value)?,
        };
        rest = after_value.trim_start().strip_prefix(',')?;
    }
}

/// Collect the keys of a JSON object whose opening brace was consumed and
/// return the text after its closing brace. Values are skipped whole, so an
/// odd value (`null`, an object) does not end the scan.
fn collect_object_keys<'a>(object: &'a str, names: &mut Vec<String>) -> Option<&'a str> {
    let mut rest = object;

    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix('}') {
            return Some(after);
        }
        let (key, after_key) = rest.strip_prefix('"').and_then(split_json_string)?;
        let value = after_key.trim_start().strip_prefix(':')?.trim_start();
        let after_value = skip_value(value)?.trim_start();

        names.push(key.to_owned());

        rest = match after_value.strip_prefix(',') {
            Some(next) => next,
            None => after_value.starts_with('}').then_some(after_value)?,
        };
    }
}

/// Skip one JSON value at the start of `value` and return the text after it.
///
/// Strings end at their closing quote, objects and arrays at the bracket that
/// closes them (brackets inside strings do not count), and anything else
/// (numbers, `true`, `null`) at the next delimiter. `None` means the value is
/// truncated or its brackets do not balance.
fn skip_value(value: &str) -> Option<&str> {
    if let Some(body) = value.strip_prefix('"') {
        return split_json_string(body).map(|(_, after)| after);
    }
    if !value.starts_with(['{', '[']) {
        return Some(
            value.trim_start_matches(|c: char| !matches!(c, ',' | '}' | ']') && !c.is_whitespace()),
        );
    }

    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in value.bytes().enumerate() {
        match byte {
            _ if escaped => escaped = false,
            b'\\' if in_string => escaped = true,
            b'"' => in_string = !in_string,
            _ if in_string => {}
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return value.get(index + 1..);
                }
            }
            _ => {}
        }
    }

    None
}

/// Split a JSON string body (after its opening quote) at the closing quote,
/// returning the raw contents and the text after the quote.
fn split_json_string(body: &str) -> Option<(&str, &str)> {
    let mut escaped = false;

    for (index, byte) in body.bytes().enumerate() {
        match byte {
            _ if escaped => escaped = false,
            b'\\' => escaped = true,
            b'"' => return Some((body.get(..index)?, body.get(index + 1..)?)),
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_dependency_and_dev_dependency_keys_only() {
        let json = r#"{
            "name": "web",
            "scripts": { "dev": "next dev", "express": "x" },
            "dependencies": { "next": "15.0.0", "react": "^19" },
            "devDependencies": {"@remix-run/dev":"2.0.0" , "eslint": "9"},
            "peerDependencies": { "vue": "3" },
            "files": ["dependencies"]
        }"#;

        let names = dependency_names(json);
        assert_eq!(names.runtime, ["next", "react"]);
        assert_eq!(names.dev, ["@remix-run/dev", "eslint"]);
        assert!(names.has_runtime(&["react"]));
        assert!(!names.has_runtime(&["eslint"]));
        assert!(names.has_any(&["eslint"]));
        assert!(!names.has_any(&["vue"]));
    }

    #[test]
    fn tolerates_escapes_odd_values_and_truncation() {
        let runtime = |json| dependency_names(json).runtime;
        assert_eq!(
            runtime(r#"{"dependencies":{"a\"b":"1","c":null,"d":"}"}}"#),
            ["a\\\"b", "c", "d"]
        );
        assert_eq!(runtime(r#"{"dependencies": {}}"#), Vec::<String>::new());
        assert_eq!(
            runtime(r#"{"dependencies": {"express": "4", "ne"#),
            ["express"]
        );
        assert_eq!(dependency_names("not json"), NodeDependencies::default());
    }

    #[test]
    fn nested_dependency_keys_do_not_count() {
        let json = r#"{
            "name": "web",
            "pnpm": {
                "packageExtensions": {
                    "react-redux@7": { "dependencies": { "express": "4" } }
                },
                "overrides": { "next": "15" }
            },
            "overrides": { "foo": { "dependencies": { "next": "15" } } },
            "workspaces": [{ "devDependencies": { "vite": "5" } }],
            "dependencies": { "react": "19" },
            "devDependencies": { "typescript": "5" }
        }"#;

        let names = dependency_names(json);
        assert_eq!(names.runtime, ["react"]);
        assert_eq!(names.dev, ["typescript"]);
    }

    #[test]
    fn braces_and_keys_inside_strings_do_not_change_the_depth() {
        let json = r#"{
            "description": "a } closing brace, \"dependencies\": {\"x\": 1} and a {",
            "scripts": { "build": "echo '{' && echo \"}\" ]", "x": "\\" },
            "config": [ "}", "]", { "k": "{[" } ],
            "dependencies": { "next": "15", "odd": { "a": "}" }, "react": "19" }
        }"#;

        let names = dependency_names(json);
        assert_eq!(names.runtime, ["next", "odd", "react"]);
        assert_eq!(names.dev, Vec::<String>::new());
    }

    #[test]
    fn only_a_top_level_object_is_read() {
        assert_eq!(
            dependency_names("\u{feff}{\"dependencies\": {\"next\": \"15\"}}").runtime,
            ["next"]
        );
        assert_eq!(
            dependency_names(r#"[{"dependencies": {"next": "15"}}]"#),
            NodeDependencies::default()
        );
        assert_eq!(
            dependency_names(r#"{"a": }}, "dependencies": {"next": "15"}}"#),
            NodeDependencies::default(),
            "a malformed value stops the scan"
        );
    }
}
