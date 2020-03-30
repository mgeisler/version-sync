use pulldown_cmark::{CodeBlockKind::Fenced, Event, Parser, Tag};
use semver_parser::range::parse as parse_request;
use semver_parser::range::VersionReq;
use semver_parser::version::parse as parse_version;
use toml::Value;

use crate::helpers::{indent, read_file, version_matches_request, Result};

/// A fenced code block.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CodeBlock {
    /// Text between the fences.
    content: String,
    /// Line number starting with 1.
    first_line: usize,
}

/// Extract a dependency on the given package from a TOML code block.
fn extract_version_request(pkg_name: &str, block: &str) -> Result<VersionReq> {
    match block.parse::<Value>() {
        Ok(value) => {
            let version = value
                .get("dependencies")
                .or_else(|| value.get("dev-dependencies"))
                .and_then(|deps| deps.get(pkg_name))
                .and_then(|dep| {
                    dep.get("version")
                        // pkg_name = { version = "1.2.3" }
                        .and_then(|version| version.as_str())
                        // pkg_name = { git = "..." }
                        .or_else(|| dep.get("git").and(Some("*")))
                        // pkg_name = "1.2.3"
                        .or_else(|| dep.as_str())
                });
            match version {
                Some(version) => parse_request(version)
                    .map_err(|err| format!("could not parse dependency: {}", err)),
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
    let parser = Parser::new(text);
    let mut code_blocks = Vec::new();
    let mut current_block = None;
    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(Fenced(lang))) if is_toml_block(&lang) => {
                // Count number of newlines before the ```. This gives
                // us the line number of the fence, counted from 0.
                let line_count = text[..range.start].chars().filter(|&ch| ch == '\n').count();
                current_block = Some(CodeBlock {
                    first_line: line_count + 2,
                    content: String::new(),
                });
            }
            Event::Text(code) => {
                if let Some(block) = current_block.as_mut() {
                    block.content.push_str(&code);
                }
            }
            Event::End(Tag::CodeBlock(_)) => {
                if let Some(block) = current_block.take() {
                    code_blocks.push(block);
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
    let text = read_file(path).map_err(|err| format!("could not read {}: {}", path, err))?;
    let version = parse_version(pkg_version)
        .map_err(|err| format!("bad package version {:?}: {}", pkg_version, err))?;

    println!("Checking code blocks in {}...", path);
    let mut failed = false;
    for block in find_toml_blocks(&text) {
        let result = extract_version_request(pkg_name, &block.content)
            .and_then(|request| version_matches_request(&version, &request));
        match result {
            Err(err) => {
                failed = true;
                println!("{} (line {}) ... {} in", path, block.first_line, err);
                println!("{}\n", indent(&block.content));
            }
            Ok(()) => println!("{} (line {}) ... ok", path, block.first_line),
        }
    }

    if failed {
        return Err(format!("dependency errors in {}", path));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_markdown_file() {
        assert_eq!(find_toml_blocks(""), vec![]);
    }

    #[test]
    fn indented_code_block() {
        assert_eq!(find_toml_blocks("    code block\n"), vec![]);
    }

    #[test]
    fn empty_toml_block() {
        assert_eq!(
            find_toml_blocks("```toml\n```"),
            vec![CodeBlock {
                content: String::new(),
                first_line: 2
            }]
        );
    }

    #[test]
    fn no_close_fence() {
        assert_eq!(
            find_toml_blocks("```toml\n"),
            vec![CodeBlock {
                content: String::new(),
                first_line: 2
            }]
        );
    }

    #[test]
    fn nonempty_toml_block() {
        let text = "Preceding text.\n\
                    ```toml\n\
                    foo\n\
                    ```\n\
                    Trailing text";
        assert_eq!(
            find_toml_blocks(&text),
            vec![CodeBlock {
                content: String::from("foo\n"),
                first_line: 3
            }]
        );
    }

    #[test]
    fn blockquote_toml_block() {
        let text = "> This is a blockquote\n\
                    >\n\
                    > ```toml\n\
                    > foo\n\
                    > \n\
                    >   bar\n\
                    >\n\
                    > ```\n\
                    ";
        assert_eq!(
            find_toml_blocks(&text),
            vec![CodeBlock {
                content: String::from("foo\n\n  bar\n\n"),
                first_line: 4
            }]
        );
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

    #[test]
    fn simple() {
        let block = "[dependencies]\n\
                     foobar = '1.5'";
        let request = extract_version_request("foobar", block);
        assert_eq!(request.unwrap(), parse_request("1.5").unwrap());
    }

    #[test]
    fn table() {
        let block = "[dependencies]\n\
                     foobar = { version = '1.5', default-features = false }";
        let request = extract_version_request("foobar", block);
        assert_eq!(request.unwrap(), parse_request("1.5").unwrap());
    }

    #[test]
    fn git_dependency() {
        // Git dependencies are translated into a "*" dependency
        // and are thus always accepted.
        let block = "[dependencies]\n\
                     foobar = { git = 'https://example.net/foobar.git' }";
        let request = extract_version_request("foobar", block);
        assert_eq!(request.unwrap(), parse_request("*").unwrap());
    }

    #[test]
    fn dev_dependencies() {
        let block = "[dev-dependencies]\n\
                     foobar = '1.5'";
        let request = extract_version_request("foobar", block);
        assert_eq!(request.unwrap(), parse_request("1.5").unwrap());
    }

    #[test]
    fn bad_version() {
        let block = "[dependencies]\n\
                     foobar = '1.5.bad'";
        let request = extract_version_request("foobar", block);
        assert_eq!(
            request.unwrap_err(),
            "could not parse dependency: \
             encountered unexpected token: AlphaNumeric(\"bad\")"
        );
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
        // toml 0.5.3 returns "found a period at line 2 column 13.
        // Update the test when we bump the toml crate dependency.
        assert!(request
            .unwrap_err()
            .contains("TOML parse error: expected newline, found a period at line 2"));
    }

    #[test]
    fn bad_path() {
        let no_such_file = if cfg!(unix) {
            "No such file or directory (os error 2)"
        } else {
            "The system cannot find the file specified. (os error 2)"
        };
        let errmsg = format!("could not read no-such-file.md: {}", no_such_file);
        assert_eq!(
            check_markdown_deps("no-such-file.md", "foobar", "1.2.3"),
            Err(errmsg)
        );
    }

    #[test]
    fn bad_pkg_version() {
        // This uses the README.md file from this crate.
        assert_eq!(
            check_markdown_deps("README.md", "foobar", "1.2"),
            Err(String::from(
                "bad package version \"1.2\": expected more input"
            ))
        );
    }
}
