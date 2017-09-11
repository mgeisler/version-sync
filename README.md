# check-versions

[![](https://travis-ci.org/mgeisler/check-versions.svg?branch=master)][travis-ci]
[![](https://ci.appveyor.com/api/projects/status/hy8camtdx5iiq26l?svg=true)][appveyor]

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
`README.md` file is kept in sync with your crate version:
```
$ cargo test --test check-versions -- --nocapture
    Finished dev [unoptimized + debuginfo] target(s) in 0.0 secs
     Running target/debug/deps/check_versions-3b40b9d452dd9385

running 1 test
Checking code blocks in README.md...
README.md (line 10) ... ok
test test_readme_deps ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Here the `README.md` file still references version 0.1.0, while the
version number in `Cargo.toml` has been changed to 0.2.0. The test
fails and the code block with the error is shown:

```
$ cargo test --test check-versions -- --nocapture
    Finished dev [unoptimized + debuginfo] target(s) in 0.0 secs
     Running target/debug/deps/check_versions-8fbc5f3b97f4ec3a

running 1 test
Checking code blocks in README.md...
README.md (line 10) ... expected minor version 2, found 1 in
    [dev-dependencies]
    check-versions = "0.1"

thread 'test_readme_deps' panicked at 'dependency errors in README.md', tests/check-versions.rs:6:4
note: Run with `RUST_BACKTRACE=1` for a backtrace.
test test_readme_deps ... FAILED

failures:

failures:
    test_readme_deps

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out

error: test failed, to rerun pass '--test check-versions'
```

## Release History

This is a changelog describing the most important changes per release.

### Version 0.1.0 — September 10th, 2017

First public release with support for finding outdated version numbers
in `dependencies` and `dev-dependencies`.

## License

Textwrap can be distributed according to the [MIT license][mit].
Contributions will be accepted under the same license.

[travis-ci]: https://travis-ci.org/mgeisler/check-versions
[appveyor]: https://ci.appveyor.com/project/mgeisler/check-versions
[mit]: LICENSE
