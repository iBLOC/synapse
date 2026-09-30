# Contributing to Synapse

Thank you for your interest in contributing to Synapse! This document
describes the contribution process and guidelines.

## Code of Conduct

By participating in this project, you agree to maintain a respectful
and inclusive environment for everyone. Be kind, professional, and
constructive in all interactions.

## How to Contribute

### Reporting Bugs

1. **Search existing issues** to avoid duplicates
2. **Use the bug report template** (if available)
3. Include:
   - Synapse version (`cargo tree` or git SHA)
   - Rust version (`rustc --version`)
   - Operating system
   - Minimal reproduction code
   - Expected vs actual behavior
   - Error messages / stack traces

### Suggesting Features

1. Open a "feature request" issue first
2. Describe the use case and motivation
3. Discuss design before implementation
4. Once approved, submit a PR

### Submitting Pull Requests

1. **Fork** the repository
2. **Create a branch** from `master`:
   ```bash
   git checkout -b feature/my-feature
   ```
3. **Make changes** following the development workflow
4. **Add tests** — all new features should have integration tests
5. **Update documentation** — user guide, API docs, specs
6. **Run checks** before pushing:
   ```bash
   cargo test
   cargo clippy --all-targets
   cargo fmt --check
   ```
7. **Commit** with clear messages (see Commit Messages below)
8. **Push** and open a PR

## Development Workflow

### Setup

```bash
# Clone
git clone https://github.com/ibloc/synapse.git
cd synapse

# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install mdbook for docs (optional)
cargo install mdbook

# Build and test
cargo build
cargo test
```

### Worktree-based Development

For isolated work, use git worktrees:

```bash
git worktree add .worktrees/my-feature -b feature/my-feature
cd .worktrees/my-feature
# ... make changes, commit ...
cd ../..
git merge feature/my-feature
git worktree remove .worktrees/my-feature
```

### Testing

All code changes should include tests:

- **Unit tests** in the same file (`#[cfg(test)] mod tests`)
- **Integration tests** in `tests/` directory
- Run full suite before pushing: `cargo test`

### Documentation

Update relevant documentation:

- **Code comments** for non-obvious logic (sparingly)
- **rustdoc** for public APIs (`///` doc comments)
- **User guide** (`docs/src/`) for usage examples
- **Design specs** (`docs/superpowers/specs/`) for architectural decisions

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short description>

<optional body>

<optional footer>
```

**Types:**
- `feat` — new feature
- `fix` — bug fix
- `refactor` — code change that neither fixes bug nor adds feature
- `test` — add or modify tests
- `docs` — documentation only
- `chore` — build/tooling changes

**Examples:**
```
feat(multiagent): HybridSupervisor combines routing + handoff
fix(transformer): MarkdownHeaderSplitter preserves deeper levels
docs: update user guide for new MCP HTTP transport
```

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag: `git tag v0.x.0`
4. Push tag: `git push --tags`
5. CI publishes to crates.io

## License

By contributing to Synapse, you agree that your contributions will be
licensed under the [Apache License 2.0](LICENSE).

## Questions?

Open a GitHub issue or discussion. We're happy to help!
