//! `version-sync` provides macros for keeping version numbers in sync
//! with your crate version.
//!
//! When making a release of a Rust project, you typically need to
//! adjust some version numbers in your code and documentation. This
//! crate gives you macros that covers the two usual cases where
//! version numbers need updating:
//!
//! * TOML examples in the `README.md` files that show how to add a
//!   dependency on your crate. See [`assert_markdown_deps_updated`].
//!
//! * The [`html_root_url`] attribute that tells other crates where to
//!   find your documentation. See [`assert_html_root_url_updated`].
//!
//! A typical configuration will use an integration test to verify
//! that all version numbers are in sync. Create a
//! `tests/version-numbers.rs` file with:
//!
//! ```rust
//! #[macro_use]
//! extern crate version_sync;
//!
//! #[test]
//! # fn fake_hidden_test_case_1() {}
//! fn test_readme_deps() {
//!     assert_markdown_deps_updated!("README.md");
//! }
//!
//! #[test]
//! # fn fake_hidden_test_case_2() {}
//! fn test_html_root_url() {
//!     assert_html_root_url_updated!("src/lib.rs");
//! }
//!
//! # fn main() {
//! #     test_readme_deps();
//! #     test_html_root_url();
//! # }
//! ```
//!
//! When you run `cargo test`, your version numbers will be
//! automatically checked.
//!
//! [`html_root_url`]: https://rust-lang-nursery.github.io/api-guidelines/documentation.html#crate-sets-html_root_url-attribute-c-html-root
//! [`assert_markdown_deps_updated`]: macro.assert_markdown_deps_updated.html
//! [`assert_html_root_url_updated`]: macro.assert_html_root_url_updated.html

#![doc(html_root_url = "https://docs.rs/version-sync/0.4.0")]
#![deny(missing_docs)]

extern crate itertools;
extern crate pulldown_cmark;
extern crate semver_parser;
extern crate syn;
extern crate toml;
extern crate url;

use std::fs::File;
use std::io::Read;
use std::result;

use pulldown_cmark::{Parser, Event, Tag};
use semver_parser::range::parse as parse_request;
use semver_parser::range::{VersionReq, Op};
use semver_parser::version::Version;
use semver_parser::version::parse as parse_version;
use toml::Value;
use url::Url;
use itertools::join;

/// The common result type, our errors will be simple strings.
type Result<T> = result::Result<T, String>;

/// A fenced code block.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CodeBlock<'a> {
    /// Text between the fences.
    content: &'a str,
    /// Line number starting with 1.
    first_line: usize,
}

impl<'a> CodeBlock<'a> {
    /// Contruct a new code block from text[start..end]. This only
    /// works for fenced code blocks. The `start` index must be the
    /// first line of data in the code block, `end` must be right
    /// after the final newline of a fenced code block.
    fn new(text: &'a str, start: usize, end: usize) -> CodeBlock {
        // A code block with no closing fence is reported as being
        // closed at the end of the file. In that case, we cannot be
        // sure to find a final newline.
        let last_nl = match text[..end - 1].rfind('\n') {
            Some(i) => i + 1,
            None => start,
        };
        let first_line = 1 + text[..start].lines().count();
        CodeBlock {
            content: &text[start..last_nl],
            first_line: first_line,
        }
    }
}

/// Return all data from `path`.
fn read_file(path: &str) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

/// Indent every line in text by four spaces.
fn indent(text: &str) -> String {
    join(text.lines().map(|line| String::from("    ") + line), "\n")
}

/// Verify that the version range request matches the given version.
fn version_matches_request(version: &Version, request: &VersionReq) -> Result<()> {
    if request.predicates.len() != 1 {
        // Can only handle simple dependencies
        return Ok(());
    }

    let pred = &request.predicates[0];
    match pred.op {
        Op::Tilde | Op::Compatible => {
            if pred.major != version.major {
                return Err(format!(
                    "expected major version {}, found {}",
                    version.major,
                    pred.major,
                ));
            }
            if let Some(minor) = pred.minor {
                if minor != version.minor {
                    return Err(format!("expected minor version {}, found {}",
                                       version.minor,
                                       minor));
                }
            }
            if let Some(patch) = pred.patch {
                if patch != version.patch {
                    return Err(format!("expected patch version {}, found {}",
                                       version.patch,
                                       patch));
                }
            }
            if pred.pre != version.pre {
                return Err(format!("expected pre-release {:?}, found {:?}",
                                   join(&version.pre, "."),
                                   join(&pred.pre, ".")));
            }
        }
        _ => return Ok(()), // We cannot check other operators.
    }

    Ok(())
}

/// Extract a dependency on the given package from a TOML code block.
fn extract_version_request(pkg_name: &str, block: &str) -> Result<VersionReq> {
    match block.parse::<Value>() {
        Ok(value) => {
            let version = value
                .get("dependencies")
                .or_else(|| value.get("dev-dependencies"))
                .and_then(|deps| deps.get(pkg_name))
                .and_then(|dep| dep.get("version").or_else(|| Some(dep)))
                .and_then(|version| version.as_str());
            match version {
                Some(version) => {
                    parse_request(version)
                        .map_err(|err| format!("could not parse dependency: {}", err))
                }
                None => Err(format!("no dependency on {}", pkg_name)),
            }
        }
        Err(err) => Err(format!("TOML parse error: {}", err)),
    }
}

/// Check if a code block language line says the block is TOML code.
fn is_toml_block(lang: &str) -> bool {
    // Split the language line as LangString::parse from rustdoc:
    // https://github.com/rust-lang/rust/blob/1.20.0/src/librustdoc/html/markdown.rs#L922
    let mut has_toml = false;
    for token in lang.split(|c: char| !(c == '_' || c == '-' || c.is_alphanumeric())) {
        match token.trim() {
            "no_sync" => return false,
            "toml" => has_toml = true,
            _ => {}
        }
    }
    has_toml
}

/// Find all TOML code blocks in a Markdown text.
fn find_toml_blocks(text: &str) -> Vec<CodeBlock> {
    let mut parser = Parser::new(text);
    let mut code_blocks = Vec::new();
    let mut start = 0;
    // A normal for-loop doesn't work since that would borrow the
    // parser mutably for the duration of the loop body, preventing us
    // from calling get_offset later.
    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                start = parser.get_offset();
            }
            Event::End(Tag::CodeBlock(lang)) => {
                // Only fenced code blocks have language information.
                if is_toml_block(&lang) {
                    let end = parser.get_offset();
                    code_blocks.push(CodeBlock::new(text, start, end));
                }
            }
            _ => {}
        }
    }

    code_blocks
}

/// Check dependencies in Markdown code blocks.
///
/// This function finds all TOML code blocks in `path` and looks for
/// dependencies on `pkg_name` in those blocks. A code block fails the
/// check if it has a dependency on `pkg_name` that doesn't match
/// `pkg_version`, or if it has no dependency on `pkg_name` at all.
///
/// # Examples
///
/// Consider a package named `foo` with version 1.2.3. The following
/// TOML block will pass the test:
///
/// ~~~markdown
/// ```toml
/// [dependencies]
/// foo = "1.2.3"
/// ```
/// ~~~
///
/// Both `dependencies` and `dev-dependencies` are examined. If you
/// want to skip a block, add `no_sync` to the language line:
///
/// ~~~markdown
/// ```toml,no_sync
/// [dependencies]
/// foo = "1.2.3"
/// ```
/// ~~~
///
/// Code blocks also fail the check if they cannot be parsed as TOML.
///
/// # Errors
///
/// If any block fails the check, an `Err` is returned with a succinct
/// error message. Status information has then already been printed on
/// `stdout`.
pub fn check_markdown_deps(path: &str, pkg_name: &str, pkg_version: &str) -> Result<()> {
    let text = read_file(path)
        .map_err(|err| format!("could not read {}: {}", path, err))?;
    let version = parse_version(pkg_version)
        .map_err(|err| format!("bad package version {:?}: {}", pkg_version, err))?;

    println!("Checking code blocks in {}...", path);
    let mut failed = false;
    for block in find_toml_blocks(&text) {
        let result = extract_version_request(pkg_name, block.content)
            .and_then(|request| version_matches_request(&version, &request));
        match result {
            Err(err) => {
                failed = true;
                println!("{} (line {}) ... {} in", path, block.first_line, err);
                println!("{}\n", indent(block.content));
            }
            Ok(()) => println!("{} (line {}) ... ok", path, block.first_line),
        }
    }

    if failed {
        return Err(format!("dependency errors in {}", path));
    }
    Ok(())
}

/// Assert that dependencies on the current package are up to date.
///
/// The macro will call [`check_markdown_deps`] on the file name given
/// in order to check that the TOML examples found all depend on a
/// current version of your package. The package name is automatically
/// taken from the `$CARGO_PKG_NAME` environment variable and the
/// version is taken from `$CARGO_PKG_VERSION`. These environment
/// variables are automatically set by Cargo when compiling your
/// crate.
///
/// # Usage
///
/// The typical way to use this macro is from an integration test:
///
/// ```rust
/// #[macro_use]
/// extern crate version_sync;
///
/// #[test]
/// # fn fake_hidden_test_case() {}
/// # // The above function ensures test_readme_deps is compiled.
/// fn test_readme_deps() {
///     assert_markdown_deps_updated!("README.md");
/// }
///
/// # fn main() {
/// #     test_readme_deps();
/// # }
/// ```
///
/// Tests are run with the current directory set to directory where
/// your `Cargo.toml` file is, so this will find a `README.md` file
/// next to your `Cargo.toml` file.
///
/// # Panics
///
/// If any TOML code block fails the check, `panic!` will be invoked.
///
/// [`check_markdown_deps`]: fn.check_markdown_deps.html
#[macro_export]
macro_rules! assert_markdown_deps_updated {
    ($path:expr) => {
        let pkg_name = env!("CARGO_PKG_NAME");
        let pkg_version = env!("CARGO_PKG_VERSION");
        if let Err(err) = $crate::check_markdown_deps($path, pkg_name, pkg_version) {
            panic!(err);
        }
    }
}

fn url_matches(value: &str, pkg_name: &str, version: &Version) -> Result<()> {
    let url = Url::parse(value)
        .map_err(|err| format!("parse error: {}", err))?;

    // Since docs.rs redirects HTTP traffic to HTTPS, we will ensure
    // that the scheme is "https" here.
    if url.scheme() != "https" {
        return Err(format!("expected \"https\", found {:?}", url.scheme()));
    }

    // We can only reason about docs.rs.
    if url.domain() != Some("docs.rs") {
        return Ok(());
    }

    let mut path_segments = url.path_segments()
        .ok_or_else(|| String::from("no path in URL"))?;

    // The package name should not be empty.
    let name = path_segments
        .next()
        .and_then(|path| if path.is_empty() { None } else { Some(path) })
        .ok_or_else(|| String::from("missing package name"))?;

    // The version number should not be empty.
    let request = path_segments
        .next()
        .and_then(|path| if path.is_empty() { None } else { Some(path) })
        .ok_or_else(|| String::from("missing version number"))?;

    // Finally, we check that the package name and version matches.
    if name != pkg_name {
        Err(format!("expected package \"{}\", found \"{}\"", pkg_name, name))
    } else {
        // The Rust API Guidelines[1] suggest using an exact version
        // number, but we have relaxed this a little and allow the
        // user to specify the version as just "1" or "1.2". We might
        // make this more strict in the future.
        //
        // [1]: https://rust-lang-nursery.github.io/api-guidelines/documentation.html
        // #crate-sets-html_root_url-attribute-c-html-root
        parse_request(request)
            .map_err(|err| format!("could not parse version in URL: {}", err))
            .and_then(|request| version_matches_request(version, &request))
    }
}

/// Check version numbers in `html_root_url` attributes.
///
/// This function parses the Rust source file in `path` and looks for
/// `html_root_url` attributes. Such an attribute must specify a valid
/// URL and if the URL points to docs.rs, it must be point to the
/// documentation for `pkg_name` and `pkg_version`.
///
/// # Errors
///
/// If any attribute fails the check, an `Err` is returned with a
/// succinct error message. Status information has then already been
/// printed on `stdout`.
pub fn check_html_root_url(path: &str, pkg_name: &str, pkg_version: &str) -> Result<()> {
    let code = read_file(path)
        .map_err(|err| format!("could not read {}: {}", path, err))?;
    let version = parse_version(pkg_version)
        .map_err(|err| format!("bad package version {:?}: {}", pkg_version, err))?;

    let krate =
        syn::parse_crate(&code)
            .map_err(|source| format!("could not parse {} with source:\n{}", path, source))?;

    println!("Checking doc attributes in {}...", path);
    for attr in krate.attrs {
        let (ident, nested_meta_items) = match attr {
            syn::Attribute {
                style: syn::AttrStyle::Inner,
                value: syn::MetaItem::List(ref ident, ref nested_meta_items),
                is_sugared_doc: false,
            } => (ident, nested_meta_items),
            _ => continue,
        };

        if ident.as_ref() != "doc" {
            continue;
        }

        for nested_meta_item in nested_meta_items {
            let meta_item = match *nested_meta_item {
                syn::NestedMetaItem::MetaItem(ref meta_item) => meta_item,
                _ => continue,
            };

            let check_result = match *meta_item {
                syn::MetaItem::NameValue(ref name, ref value) if name == "html_root_url" => {
                    match *value {
                        // Accept both cooked and raw strings here.
                        syn::Lit::Str(ref s, _) => url_matches(s, pkg_name, &version),
                        // A non-string html_root_url is probably an
                        // error, but we leave this check to the
                        // compiler.
                        _ => continue,
                    }
                }
                syn::MetaItem::Word(ref name) if name == "html_root_url" => {
                    Err(String::from("html_root_url attribute without URL"))
                }
                _ => continue,
            };

            match check_result {
                Ok(()) => {
                    // FIXME: re-add line numbers and position in line
                    // when the syn crate have enough capabilities to
                    // do so.
                    println!("{} ... ok", path);
                    return Ok(());
                }
                Err(err) => {
                    println!("{} ... {}", path, err);
                    return Err(format!("html_root_url errors in {}", path));
                }
            }
        }
    }

    Ok(())
}

/// Assert that the `html_root_url` attribute is up to date.
///
/// Library code is [expected to set `html_root_url`][api-guidelines]
/// to point to docs.rs so that rustdoc can generate correct links
/// when referring to this crate.
///
/// The macro will call [`check_html_root_url`] on the file name given
/// in order to check that the `html_root_url` is points to the
/// current version of your package documentation on docs.rs. The
/// package name is automatically taken from the `$CARGO_PKG_NAME`
/// environment variable and the version is taken from
/// `$CARGO_PKG_VERSION`. These environment variables are
/// automatically set by Cargo when compiling your crate.
///
/// # Usage
///
/// The typical way to use this macro is from an integration test:
///
/// ```rust
/// #[macro_use]
/// extern crate version_sync;
///
/// #[test]
/// # fn fake_hidden_test_case() {}
/// # // The above function ensures test_html_root_url is compiled.
/// fn test_html_root_url() {
///     assert_html_root_url_updated!("src/lib.rs");
/// }
///
/// # fn main() {
/// #     test_html_root_url();
/// # }
/// ```
///
/// Tests are run with the current directory set to directory where
/// your `Cargo.toml` file is, so this will find the `src/lib.rs`
/// crate root.
///
/// # Panics
///
/// If the `html_root_url` fails the check, `panic!` will be invoked.
///
/// [api-guidelines]: https://rust-lang-nursery.github.io/api-guidelines/documentation.html#crate-sets-html_root_url-attribute-c-html-root
/// [`check_html_root_url`]: fn.check_html_root_url.html
#[macro_export]
macro_rules! assert_html_root_url_updated {
    ($path:expr) => {
        let pkg_name = env!("CARGO_PKG_NAME");
        let pkg_version = env!("CARGO_PKG_VERSION");
        if let Err(err) = $crate::check_html_root_url($path, pkg_name, pkg_version) {
            panic!(err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_block_new() {
        let text = "Preceding text.\n\
                    ```\n\
                    foo\n\
                    ```\n\
                    Trailing text";
        let start = text.find("```\n").unwrap() + 4;
        let end = text.rfind("```\n").unwrap() + 4;
        assert_eq!(CodeBlock::new(text, start, end),
                   CodeBlock { content: "foo\n", first_line: 3 });
    }

    #[test]
    fn is_toml_block_simple() {
        assert!(!is_toml_block("rust"));
    }

    #[test]
    fn is_toml_block_comma() {
        assert!(is_toml_block("foo,toml"));
    }

    #[test]
    fn is_toml_block_no_sync() {
        assert!(!is_toml_block("toml,no_sync"));
        assert!(!is_toml_block("toml, no_sync"));
    }

    mod test_version_matches_request {
        use super::*;

        #[test]
        fn implicit_compatible() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn compatible() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("^1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn tilde() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("~1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn no_patch() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("1.2").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn no_minor() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("1").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn multiple_predicates() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request(">= 1.2.3, < 2.0").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn unhandled_operator() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("< 2.0").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn bad_major() {
            let version = parse_version("2.0.0").unwrap();
            let request = parse_request("1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request),
                       Err(String::from("expected major version 2, found 1")));
        }

        #[test]
        fn bad_minor() {
            let version = parse_version("1.3.0").unwrap();
            let request = parse_request("1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request),
                       Err(String::from("expected minor version 3, found 2")));
        }

        #[test]
        fn bad_patch() {
            let version = parse_version("1.2.4").unwrap();
            let request = parse_request("1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request),
                       Err(String::from("expected patch version 4, found 3")));
        }

        #[test]
        fn bad_pre_release() {
            let version = parse_version("1.2.3-rc2").unwrap();
            let request = parse_request("1.2.3-rc1").unwrap();
            assert_eq!(version_matches_request(&version, &request),
                       Err(String::from("expected pre-release \"rc2\", found \"rc1\"")));
        }
    }

    mod test_extract_version_request {
        use super::*;

        #[test]
        fn simple() {
            let block = "[dependencies]\n\
                         foobar = '1.5'";
            let request = extract_version_request("foobar", block);
            assert_eq!(request.unwrap().predicates,
                       parse_request("1.5").unwrap().predicates);
        }

        #[test]
        fn table() {
            let block = "[dependencies]\n\
                         foobar = { version = '1.5', default-features = false }";
            let request = extract_version_request("foobar", block);
            assert_eq!(request.unwrap().predicates,
                       parse_request("1.5").unwrap().predicates);
        }

        #[test]
        fn dev_dependencies() {
            let block = "[dev-dependencies]\n\
                         foobar = '1.5'";
            let request = extract_version_request("foobar", block);
            assert_eq!(request.unwrap().predicates,
                       parse_request("1.5").unwrap().predicates);
        }

        #[test]
        fn bad_version() {
            let block = "[dependencies]\n\
                         foobar = '1.5.bad'";
            let request = extract_version_request("foobar", block);
            assert_eq!(request.unwrap_err(),
                       "could not parse dependency: Extra junk after valid predicate: .bad");
        }

        #[test]
        fn missing_dependency() {
            let block = "[dependencies]\n\
                         baz = '1.5.8'";
            let request = extract_version_request("foobar", block);
            assert_eq!(request.unwrap_err(), "no dependency on foobar");
        }

        #[test]
        fn empty() {
            let request = extract_version_request("foobar", "");
            assert_eq!(request.unwrap_err(), "no dependency on foobar");
        }

        #[test]
        fn bad_toml() {
            let block = "[dependencies]\n\
                         foobar = 1.5.8";
            let request = extract_version_request("foobar", block);
            assert_eq!(request.unwrap_err(),
                       "TOML parse error: expected newline, found a period at line 2");
        }
    }

    mod test_find_toml_blocks {
        use super::*;

        #[test]
        fn empty() {
            assert_eq!(find_toml_blocks(""), vec![]);
        }

        #[test]
        fn indented_block() {
            assert_eq!(find_toml_blocks("    code block\n"), vec![]);
        }

        #[test]
        fn single() {
            assert_eq!(find_toml_blocks("```toml\n```"),
                       vec![CodeBlock { content: "", first_line: 2 }]);
        }

        #[test]
        fn no_close_fence() {
            assert_eq!(find_toml_blocks("```toml\n"),
                       vec![CodeBlock { content: "", first_line: 2 }]);
        }
    }

    mod test_url_matches {
        use super::*;

        #[test]
        fn good_url() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/foo/1.2.3", "foo", &ver),
                       Ok(()));
        }

        #[test]
        fn trailing_slash() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/foo/1.2.3/", "foo", &ver),
                       Ok(()));
        }

        #[test]
        fn without_patch() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/foo/1.2", "foo", &ver), Ok(()));
        }

        #[test]
        fn without_minor() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/foo/1", "foo", &ver), Ok(()));
        }

        #[test]
        fn different_domain() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://example.net/foo/", "bar", &ver), Ok(()));
        }

        #[test]
        fn http_url() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("http://docs.rs/foo/1.2.3", "foo", &ver),
                       Err(String::from("expected \"https\", found \"http\"")));
        }

        #[test]
        fn bad_scheme() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("mailto:foo@example.net", "foo", &ver),
                       Err(String::from("expected \"https\", found \"mailto\"")));
        }

        #[test]
        fn no_package() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs", "foo", &ver),
                       Err(String::from("missing package name")));
        }

        #[test]
        fn no_package_trailing_slash() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/", "foo", &ver),
                       Err(String::from("missing package name")));
        }

        #[test]
        fn no_version() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/foo", "foo", &ver),
                       Err(String::from("missing version number")));
        }

        #[test]
        fn no_version_trailing_slash() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/foo/", "foo", &ver),
                       Err(String::from("missing version number")));
        }

        #[test]
        fn bad_url() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("docs.rs/foo/bar", "foo", &ver),
                       Err(String::from("parse error: relative URL without a base")));
        }

        #[test]
        fn bad_pkg_version() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/foo/1.2.bad/", "foo", &ver),
                       Err(String::from("could not parse version in URL: \
                                         Extra junk after valid predicate: .bad")));
        }

        #[test]
        fn wrong_pkg_name() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/foo/1.2.3/", "bar", &ver),
                       Err(String::from("expected package \"bar\", found \"foo\"")));
        }
    }

    mod test_check_markdown_deps {
        use super::*;

        #[test]
        fn bad_path() {
            let no_such_file = if cfg!(unix) {
                "No such file or directory (os error 2)"
            } else {
                "The system cannot find the file specified. (os error 2)"
            };
            let errmsg = format!("could not read no-such-file.md: {}", no_such_file);
            assert_eq!(check_markdown_deps("no-such-file.md", "foobar", "1.2.3"),
                       Err(errmsg));
        }

        #[test]
        fn bad_pkg_version() {
            // This uses the README.md file from this crate.
            assert_eq!(check_markdown_deps("README.md", "foobar", "1.2"),
                       Err(String::from("bad package version \"1.2\": \
                                         Expected dot")));
        }
    }

    mod test_check_html_root_url {
        use super::*;

        #[test]
        fn bad_path() {
            let no_such_file = if cfg!(unix) {
                "No such file or directory (os error 2)"
            } else {
                "The system cannot find the file specified. (os error 2)"
            };
            let errmsg = format!("could not read no-such-file.md: {}", no_such_file);
            assert_eq!(check_html_root_url("no-such-file.md", "foobar", "1.2.3"),
                       Err(errmsg));
        }

        #[test]
        fn bad_pkg_version() {
            // This uses the src/lib.rs file from this crate.
            assert_eq!(check_html_root_url("src/lib.rs", "foobar", "1.2"),
                       Err(String::from("bad package version \"1.2\": \
                                         Expected dot")));
        }
    }
}
