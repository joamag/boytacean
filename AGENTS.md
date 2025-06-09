# Development Guide

This document describes how to work with the project. Follow these notes when writing code or submitting pull requests.

## Setup

Install Python packages and the Rust toolchain:

```bash
pip install -r requirements.txt
rustup default nightly
rustup component add rustfmt
rustup component add clippy
cargo install cargo-vcpkg
```

## Formatting

Format all code before committing:

```bash
cargo fmt --all
cargo clippy --fix --allow-dirty --allow-staged --all-features --all-targets
black .
```

## Testing

Run the full test suite:

```bash
cargo test --all-features
```

## Style Guide

- Always update `CHANGELOG.md` according to semantic versioning, mentioning your changes in the unreleased section.
- Write commit messages using [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
- Never bump the internal package version in `Cargo.toml` or `setup.py`. This is handled automatically by the release process.
- Rust files use LF line endings, while Python files use CRLF.

## License

Boytacean is licensed under the [Apache License, Version 2.0](http://www.apache.org/licenses/).
