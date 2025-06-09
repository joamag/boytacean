# Development Guide

This document describes how to work with the project. Follow these notes when writing code or submitting pull requests.

## Setup

Install Python packages and the Rust toolchain:

```bash
pip install -r requirements.txt
rustup default stable
rustup component add rustfmt
rustup component add clippy
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

- Always update `CHANGELOG.md` according to semantic versioning, mentioning your changes.
- Write commit messages using [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).

## License

Boytacean is licensed under the [Apache License, Version 2.0](http://www.apache.org/licenses/).
