# Contributing to TrinityChain

Thank you for your interest in contributing to TrinityChain! This document provides guidelines and instructions for getting started.

## Code of Conduct

Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before contributing. We expect all contributors to uphold these standards.

## Getting Started

### Prerequisites

- **Rust 1.70+** (install via [rustup](https://rustup.rs/))
- **Git**
- **SQLite** (usually included on most systems)

### Setting Up Your Development Environment

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/TrinityChain.git
   cd TrinityChain
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/TrinityChain/TrinityChain.git
   ```
4. Create a branch for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

### Building and Testing

Run the test suite to verify everything builds and passes:

```bash
cargo test --lib
```

Format your code before committing:

```bash
cargo fmt
```

Run clippy for linting suggestions:

```bash
cargo clippy --all-targets
```

## Making Changes

### Commit Guidelines

- Write clear, concise commit messages
- Use the imperative mood ("Add feature" not "Added feature")
- Reference issues where relevant: `Fixes #123`
- Keep commits atomic (one logical change per commit)

Example:
```
feat: add parallel mining support with configurable thread count

Implement mine_block_parallel() function using Rayon for thread-pool
based nonce search. Users can now pass --threads N to mining CLIs.

Fixes #42
```

### Code Style

- Follow Rust conventions (use `cargo fmt`)
- Add documentation comments for public APIs
- Include unit tests for new functionality
- Keep functions focused and testable

### Testing

- Add tests for all new features and bug fixes
- Ensure all tests pass: `cargo test --lib`
- Test your changes manually if applicable

## Submitting a Pull Request

1. Push your branch to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```

2. Open a Pull Request (PR) on GitHub with:
   - A clear, descriptive title
   - A description of the changes (template provided)
   - Reference to any related issues
   - Evidence of testing (test results, manual verification)

3. Respond to code review feedback promptly and professionally

4. Once approved, your PR will be merged by a maintainer

## Areas for Contribution

We welcome contributions in many areas:

- **Networking**: P2P reliability, NAT traversal, message schema improvements
- **Mining**: Optimization of parallel mining, thread configuration
- **Mempool & Fees**: Fee estimation improvements, transaction propagation
- **API & UX**: REST API expansion, block explorer enhancements
- **Security**: Audits, rate limiting, peer authentication
- **Documentation**: Guides, architecture diagrams, examples
- **Testing**: Additional test coverage, integration tests

See [DEVELOPMENT_PLAN.md](documentation/DEVELOPMENT_PLAN.md) for larger ongoing projects.

## Reporting Bugs

If you discover a bug:

1. Check if it's already reported in [GitHub Issues](https://github.com/TrinityChain/TrinityChain/issues)
2. If not, open a new issue with:
   - Clear title and description
   - Steps to reproduce
   - Expected vs. actual behavior
   - Rust version, OS, and other relevant environment details

See our Issue template for details.

## Reporting Security Issues

**Do not open public GitHub issues for security vulnerabilities.** See [SECURITY.md](SECURITY.md) for responsible disclosure instructions.

## Questions and Discussions

- Open a GitHub Discussion or Issue for questions
- Join our community chat (if available)
- Check existing documentation in `/documentation` folder

## License

By contributing to TrinityChain, you agree that your contributions will be licensed under the same license as the project (see [LICENSE](LICENSE)).

Thank you for making TrinityChain better! 🔺
