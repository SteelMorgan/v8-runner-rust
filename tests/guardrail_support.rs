use std::fs;
use std::path::{Path, PathBuf};

pub fn production_rust_contents(path: &Path) -> String {
    let contents = fs::read_to_string(path).expect("read source");
    strip_test_only_sections(&contents)
}

pub fn collect_rust_files(root: &Path) -> Vec<PathBuf> {
    fn visit(dir: &Path, files: &mut Vec<PathBuf>) {
        let mut entries = fs::read_dir(dir)
            .expect("read_dir")
            .map(|entry| entry.expect("dir entry").path())
            .collect::<Vec<_>>();
        entries.sort();

        for entry in entries {
            if entry.is_dir() {
                visit(&entry, files);
            } else if entry.extension().is_some_and(|extension| extension == "rs") {
                files.push(entry);
            }
        }
    }

    let mut files = Vec::new();
    visit(root, &mut files);
    files
}

#[allow(dead_code)]
pub fn extract_function_block<'a>(contents: &'a str, fn_name: &str) -> &'a str {
    let start_pattern = format!("fn {fn_name}");
    let start = contents
        .find(&start_pattern)
        .unwrap_or_else(|| panic!("missing function {fn_name}"));
    let open_brace = contents[start..]
        .find('{')
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("missing opening brace for function {fn_name}"));
    let end = matching_brace(contents, open_brace)
        .unwrap_or_else(|| panic!("missing closing brace for function {fn_name}"));
    &contents[start..=end]
}

fn strip_test_only_sections(contents: &str) -> String {
    const TEST_ATTR: &str = "#[cfg(test)]";

    let mut output = String::with_capacity(contents.len());
    let mut cursor = 0;
    while let Some(offset) = contents[cursor..].find(TEST_ATTR) {
        let attr_start = cursor + offset;
        output.push_str(&contents[cursor..attr_start]);
        cursor = skip_cfg_test_item(contents, attr_start + TEST_ATTR.len());
    }
    output.push_str(&contents[cursor..]);
    output
}

fn skip_cfg_test_item(contents: &str, cursor: usize) -> usize {
    let mut cursor = skip_whitespace(contents, cursor);

    while contents[cursor..].starts_with("#[") {
        cursor = next_line_start(contents, cursor);
        cursor = skip_whitespace(contents, cursor);
    }

    let remainder = &contents[cursor..];
    let next_semicolon = remainder.find(';').map(|offset| cursor + offset);
    let next_open_brace = remainder.find('{').map(|offset| cursor + offset);

    match (next_open_brace, next_semicolon) {
        (Some(open_brace), Some(semicolon)) if semicolon < open_brace => semicolon + 1,
        (None, Some(semicolon)) => semicolon + 1,
        (Some(open_brace), _) => matching_brace(contents, open_brace)
            .map(|close_brace| close_brace + 1)
            .unwrap_or(contents.len()),
        (None, None) => contents.len(),
    }
}

fn skip_whitespace(contents: &str, mut cursor: usize) -> usize {
    while cursor < contents.len() {
        let ch = contents[cursor..]
            .chars()
            .next()
            .expect("cursor must point to valid utf-8");
        if !ch.is_whitespace() {
            break;
        }
        cursor += ch.len_utf8();
    }
    cursor
}

fn next_line_start(contents: &str, cursor: usize) -> usize {
    contents[cursor..]
        .find('\n')
        .map(|offset| cursor + offset + 1)
        .unwrap_or(contents.len())
}

fn matching_brace(contents: &str, open_brace: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, byte) in contents.as_bytes()[open_brace..].iter().enumerate() {
        match *byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(open_brace + offset);
                }
            }
            _ => {}
        }
    }
    None
}
