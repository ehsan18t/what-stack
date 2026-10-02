//! Dependency names from `package.json` without a JSON parser.
//!
//! Only the keys of `"dependencies"` and `"devDependencies"` objects are
//! needed, and those objects are flat maps from package name to version
//! string. A small scanner over the (capped, already decoded) text finds them
//! without adding a JSON dependency to the crate.

/// Keys whose object values list a package's dependencies.
const DEPENDENCY_KEYS: &[&str] = &["\"dependencies\"", "\"devDependencies\""];

/// Package names listed under `"dependencies"` or `"devDependencies"`.
///
/// Every occurrence of either key that is followed by `:` and an object is
/// read, so a nested object with the same key also counts. Malformed or
/// truncated objects yield the names read before the problem.
pub fn dependency_names(json: &str) -> Vec<String> {
    let mut names = Vec::new();

    for key in DEPENDENCY_KEYS {
        for (index, _) in json.match_indices(key) {
            let object = json
                .get(index + key.len()..)
                .and_then(|rest| rest.trim_start().strip_prefix(':'))
                .and_then(|rest| rest.trim_start().strip_prefix('{'));

            if let Some(object) = object {
                collect_object_keys(object, &mut names);
            }
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

        assert_eq!(
            dependency_names(json),
            ["next", "react", "@remix-run/dev", "eslint"]
        );
    }

    #[test]
    fn tolerates_escapes_odd_values_and_truncation() {
        assert_eq!(
            dependency_names(r#"{"dependencies":{"a\"b":"1","c":null,"d":"}"}}"#),
            ["a\\\"b", "c", "d"]
        );
        assert_eq!(
            dependency_names(r#"{"dependencies": {}}"#),
            Vec::<String>::new()
        );
        assert_eq!(
            dependency_names(r#"{"dependencies": {"express": "4", "ne"#),
            ["express"]
        );
        assert_eq!(dependency_names("not json"), Vec::<String>::new());
    }
}
