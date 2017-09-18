extern crate pulldown_cmark;
extern crate toml;
extern crate semver_parser;

use std::fs::File;
use std::io::Read;
use std::result;

use pulldown_cmark::{Parser, Event, Tag};
use semver_parser::range::parse as parse_request;
use semver_parser::range::{VersionReq, Op};
use semver_parser::version::Version;
use semver_parser::version::parse as parse_version;
use toml::Value;

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
    text.lines()
        .map(|line| String::from("    ") + line)
        .collect::<Vec<_>>()
        .join("\n")
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
    lang.split(|c: char| !(c == '_' || c == '-' || c.is_alphanumeric()))
        .any(|token| token.trim() == "toml")
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
/// Code blocks also fails the check if they cannot be parsed as TOML.
///
/// # Errors
///
/// If any block failed the check, an `Err` is returned that can be
/// used to make a test fail or pass.
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
/// ```rust,no_run
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
/// # fn main() {}
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
        assert!(!is_toml_block("rust"))
    }

    #[test]
    fn is_toml_block_comma() {
        assert!(is_toml_block("foo,toml"))
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
}
