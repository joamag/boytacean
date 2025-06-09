# Agents.md file

This document serves as the main reference for the agent's configuration, usage, and development notes. Please refer to the sections below for detailed instructions and guidelines.

## Configuration

### Rust Configuration

Use the nightly version of Rust for development. This will ensure that all features are compilable.

```bash
rustup default nightly
rustup add component rustfmt
rustup add component clippy
cargo install cargo-vcpkg
```

## Formatting

Always format the code before commiting using, making sure that the Rust code is properly formatted using:

```bash
cargo fmt --all
```

For the Python code, use:

```bash
black .
```

## Testing

Always run the tests before committing changes to ensure that everything is working as expected. Use the following commands:

```bash
cargo test --all
```

## Style Guide

### Changelog

It's important to keep track of changes made to the codebase. Use proper semantic versioning and document changes in the `CHANGELOG.md` file.

### Commit Messages

When committing changes, use clear and descriptive commit messages that explain the purpose of the changes. The commit messages should be concise yet informative, providing context for future reference.

Also the commit messages should follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) standard to ensure clarity and consistency.

## License

Boytacean is licensed under the [Apache License, Version 2.0](http://www.apache.org/licenses/).
