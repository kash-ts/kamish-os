# Pre-commit hooks

This repository uses [pre-commit](https://pre-commit.com/) to run lightweight checks on staged files before each commit. The hooks check whitespace, file endings, YAML/JSON/TOML syntax, merge markers, filename case conflicts, and Rust formatting.

## Install

Install the `pre-commit` command using your preferred Python environment, for example:

```bash
python3 -m pip install --user pre-commit
```

From the repository root, install the Git hook and initialize its pinned hook environments:

```bash
pre-commit install
pre-commit run --all-files
```

After installation, checks run automatically on `git commit`. To run them manually on staged files:

```bash
pre-commit run
```

Rust formatting checks run when Rust files are staged and require the Rust toolchain. The complete CI test suite remains in GitHub Actions; pre-commit is intended for fast local feedback, not as a replacement for CI.
