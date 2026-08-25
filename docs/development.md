# Development and releases

## Local setup

```bash
cargo fmt --check
cargo test --all-targets
cargo package --allow-dirty
```

Build the docs locally with:

```bash
uvx --with mkdocs-material mkdocs build --strict
uvx --with mkdocs-material mkdocs serve
```

Open `http://127.0.0.1:8000/` while editing.

## Repository layout

```text
src/lib.rs          public crate and backend traits
tests/              SQLite integration tests
examples/           runnable generic example
docs/               MkDocs documentation
.github/workflows/  CI, release, and Pages publishing
```

## CI and documentation

Every push runs formatting and all Rust targets. The documentation workflow
builds the MkDocs site with `--strict` and deploys it to GitHub Pages without
creating a documentation branch.

## crates.io release

1. Update `version` in `Cargo.toml` and the changelog.
2. Run the local check set and `cargo package`.
3. Create and push a version tag, then publish the matching GitHub Release.
4. The release workflow publishes through `CARGO_REGISTRY_TOKEN`.

The token should be scoped to this crate and limited to publishing. Do not put
it in the repository or in documentation.
