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
    use proptest::prelude::*;

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

    /// A generated JSON value for the scanner property test.
    #[derive(Clone, Debug)]
    enum Json {
        Null,
        Bool(bool),
        Number(i64),
        Text(String),
        Array(Vec<Self>),
        Object(Vec<(String, Self)>),
    }

    /// Serializes generated values with varied whitespace between tokens.
    struct Writer {
        out: String,
        spaces: Vec<&'static str>,
        next_space: usize,
    }

    impl Writer {
        fn space(&mut self) {
            if !self.spaces.is_empty() {
                self.out
                    .push_str(self.spaces[self.next_space % self.spaces.len()]);
                self.next_space += 1;
            }
        }

        fn string(&mut self, text: &str) {
            self.out.push('"');
            self.out.push_str(&escape(text));
            self.out.push('"');
        }

        fn value(&mut self, value: &Json) {
            match value {
                Json::Null => self.out.push_str("null"),
                Json::Bool(flag) => self.out.push_str(if *flag { "true" } else { "false" }),
                Json::Number(number) => self.out.push_str(&number.to_string()),
                Json::Text(text) => self.string(text),
                Json::Array(items) => {
                    self.out.push('[');
                    for (index, item) in items.iter().enumerate() {
                        if index > 0 {
                            self.out.push(',');
                        }
                        self.space();
                        self.value(item);
                    }
                    self.space();
                    self.out.push(']');
                }
                Json::Object(entries) => self.object(entries),
            }
        }

        fn object(&mut self, entries: &[(String, Json)]) {
            self.out.push('{');
            for (index, (key, value)) in entries.iter().enumerate() {
                if index > 0 {
                    self.out.push(',');
                }
                self.space();
                self.string(key);
                self.space();
                self.out.push(':');
                self.space();
                self.value(value);
            }
            self.space();
            self.out.push('}');
        }
    }

    /// JSON string body for `text`: the raw form the scanner returns.
    fn escape(text: &str) -> String {
        use std::fmt::Write as _;

        let mut escaped = String::new();
        for c in text.chars() {
            match c {
                '"' => escaped.push_str("\\\""),
                '\\' => escaped.push_str("\\\\"),
                '\n' => escaped.push_str("\\n"),
                c if c.is_control() => {
                    write!(escaped, "\\u{:04x}", u32::from(c)).expect("write to a String");
                }
                c => escaped.push(c),
            }
        }
        escaped
    }

    /// Short text full of characters that matter to the scanner.
    fn tricky_text() -> impl Strategy<Value = String> {
        "[a-z@/._ {}\\[\\]\":,\\\\\n\u{e9}\u{1f4a5}-]{0,10}"
    }

    /// Object keys, including the dependency keys so they appear nested.
    fn any_key() -> impl Strategy<Value = String> {
        prop_oneof![
            3 => tricky_text(),
            1 => Just("dependencies".to_owned()),
            1 => Just("devDependencies".to_owned()),
        ]
    }

    fn any_json() -> impl Strategy<Value = Json> {
        let leaf = prop_oneof![
            Just(Json::Null),
            any::<bool>().prop_map(Json::Bool),
            any::<i64>().prop_map(Json::Number),
            tricky_text().prop_map(Json::Text),
        ];
        leaf.prop_recursive(3, 32, 4, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..4).prop_map(Json::Array),
                prop::collection::vec((any_key(), inner), 0..4).prop_map(Json::Object),
            ]
        })
    }

    /// A dependency map: package names with version strings.
    fn dependency_map() -> impl Strategy<Value = Option<Vec<(String, String)>>> {
        prop::option::of(prop::collection::vec((tricky_text(), tricky_text()), 0..6))
    }

    fn dependency_entry(map: &[(String, String)]) -> Json {
        Json::Object(
            map.iter()
                .map(|(name, version)| (name.clone(), Json::Text(version.clone())))
                .collect(),
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        #[test]
        fn scanner_returns_exactly_the_top_level_dependency_names(
            others in prop::collection::vec(
                (tricky_text().prop_filter("not a dependency key", |key| {
                    key != "dependencies" && key != "devDependencies"
                }), any_json()),
                0..6,
            ),
            runtime in dependency_map(),
            dev in dependency_map(),
            positions in (any::<prop::sample::Index>(), any::<prop::sample::Index>()),
            spaces in prop::collection::vec(prop::sample::select(&["", " ", "\n  ", "\t", "\r\n"][..]), 0..4),
            bom in any::<bool>(),
        ) {
            let mut entries = others;
            if let Some(map) = &runtime {
                let at = positions.0.index(entries.len() + 1);
                entries.insert(at, ("dependencies".to_owned(), dependency_entry(map)));
            }
            if let Some(map) = &dev {
                let at = positions.1.index(entries.len() + 1);
                entries.insert(at, ("devDependencies".to_owned(), dependency_entry(map)));
            }

            let mut writer = Writer { out: String::new(), spaces, next_space: 0 };
            if bom {
                writer.out.push('\u{feff}');
            }
            writer.object(&entries);

            let names = |map: Option<Vec<(String, String)>>| -> Vec<String> {
                map.unwrap_or_default().iter().map(|(name, _)| escape(name)).collect()
            };
            let expected = NodeDependencies { runtime: names(runtime), dev: names(dev) };
            prop_assert_eq!(dependency_names(&writer.out), expected, "{}", writer.out);
        }
    }
}
