#[test]
fn test_readme_deps() {
    version_sync::assert_markdown_deps_updated!("README.md");
}

#[test]
fn test_readme_changelog() {
    version_sync::assert_contains_regex!(
        "README.md",
        r"^### Version {version} — .* \d\d?.., 20\d\d$"
    );
}

#[test]
fn test_minimum_rustc_version() {
    let version = r"1\.31\.0";
    version_sync::assert_contains_regex!(".travis.yml", &format!(r"^  - {}", version));
    version_sync::assert_contains_regex!("README.md", &format!("badge/rustc-{}", version));
}

#[test]
fn test_html_root_url() {
    version_sync::assert_html_root_url_updated!("src/lib.rs");
}
