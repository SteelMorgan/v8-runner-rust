mod guardrail_support;

use std::path::Path;

use guardrail_support::{collect_rust_files, production_rust_contents};

const FORBIDDEN_PATTERNS: &[&str] = &["clap::", "crate::cli::", "crate::output::", "crate::mcp::"];

fn assert_missing(path: &Path, forbidden: &str) {
    let production = production_rust_contents(path);
    assert!(
        !production.contains(forbidden),
        "{} must not import {}",
        path.display(),
        forbidden
    );
}

#[test]
fn use_cases_do_not_depend_on_transport_or_presentation_types() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("use_cases");
    let files = collect_rust_files(&root);
    assert!(
        files.len() >= 10,
        "expected to scan the full use-case layer, found only {} files",
        files.len()
    );

    for file in &files {
        for forbidden in FORBIDDEN_PATTERNS {
            assert_missing(file, forbidden);
        }
    }
}
