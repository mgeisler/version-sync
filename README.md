# check-versions

The `check-versions` crate is a simple crate that will help you
remember to update the versions numbers in your `README.md` file.

## Usage

Add this to your `Cargo.toml`:
```toml
[dev-dependencies]
check-versions = "0.1"
```

Then create a `tests/check-versions.rs` with:
```rust
#[macro_use]
extern crate check_versions;

#[test]
fn test_readme_deps() {
    assert_markdown_deps_updated!("README.md");
}
```

This test will ensure that the dependencies mentioned in your
`README.md` file is kept in sync with your crate version.

## Release History

This is a changelog describing the most important changes per release.

### Version 0.1.0 — September 10th, 2017

First public release with support for finding outdated version numbers
in `dependencies` and `dev-dependencies`.

## License

Textwrap can be distributed according to the [MIT license][mit].
Contributions will be accepted under the same license.

[mit]: LICENSE
