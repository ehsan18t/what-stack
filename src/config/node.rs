//! Dependency names from `package.json` without a JSON parser.
//!
//! Only the keys of `"dependencies"` and `"devDependencies"` objects are
//! needed, and those objects are flat maps from package name to version
//! string. A small scanner over the (capped, already decoded) text finds them
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

/// Package names listed under `"dependencies"` and `"devDependencies"`.
///
/// Every occurrence of either key that is followed by `:` and an object is
/// read, so a nested object with the same key also counts. Malformed or
/// truncated objects yield the names read before the problem.
pub fn dependency_names(json: &str) -> NodeDependencies {
    NodeDependencies {
        runtime: names_under(json, "\"dependencies\""),
        dev: names_under(json, "\"devDependencies\""),
    }
}

fn names_under(json: &str, key: &str) -> Vec<String> {
    let mut names = Vec::new();

    for (index, _) in json.match_indices(key) {
        let object = json
            .get(index + key.len()..)
            .and_then(|rest| rest.trim_start().strip_prefix(':'))
            .and_then(|rest| rest.trim_start().strip_prefix('{'));

        if let Some(object) = object {
            collect_object_keys(object, &mut names);
        }
    }

    names
}

/// Collect the keys of a flat JSON object whose opening brace was consumed.
fn collect_object_keys(object: &str, names: &mut Vec<String>) {
    let mut rest = object;

    loop {
        let Some((key, after_key)) = rest
            .trim_start()
            .strip_prefix('"')
            .and_then(split_json_string)
        else {
            return;
        };
        let Some(value) = after_key.trim_start().strip_prefix(':') else {
            return;
        };
        let value = value.trim_start();
        let after_value = match value.strip_prefix('"') {
            Some(string) => match split_json_string(string) {
                Some((_, after)) => after,
                None => return,
            },
            None => value.trim_start_matches(|c| c != ',' && c != '}'),
        };

        names.push(key.to_owned());

        match after_value.trim_start().strip_prefix(',') {
            Some(next) => rest = next,
            None => return,
        }
    }
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
}
