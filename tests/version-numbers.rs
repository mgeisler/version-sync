#[test]
#[cfg(feature = "markdown_deps_updated")]
fn test_readme_deps() {
    version_sync::assert_markdown_deps_updated!("README.md");
}

#[test]
#[cfg(feature = "contains_regex")]
fn test_readme_changelog() {
    version_sync::assert_contains_regex!(
        "README.md",
        r"^### Version {version} — 20\d\d-\d\d-\d\d$"
    );
}

#[test]
#[cfg(feature = "contains_regex")]
fn test_readme_links_are_updated() {
    version_sync::assert_only_contains_regex!("README.md", "docs.rs/{name}/{version}/");
}

#[test]
#[cfg(feature = "html_root_url_updated")]
fn test_html_root_url() {
    version_sync::assert_html_root_url_updated!("src/lib.rs");
}
