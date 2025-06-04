# Contributing to poDTest

Thank you for considering contributing to **poDTest**, a Rust-based CLI tool for building and testing Docker images in CI/CD pipelines. We welcome contributions from the community to improve the tool, whether through bug fixes, new features, documentation, or GitHub Action enhancements. This guide outlines how to contribute effectively.

## Getting Started

### 1. **Read the README**:
   * Review the README.md for an overview of `poDTest`, its features, and setup instructions.
   * Ensure you understand the tool's purpose: automating Docker image builds, health checks, and cleanup.

### 2. **Set Up the Development Environment**:
   * **Prerequisites**:
      * Rust 1.65 or higher (install via rustup).
      * Docker (see Docker installation).
      * Git for cloning and version control.
   * **Clone the Repository**:

```bash
git clone https://github.com/h4nz0x/poDTest.git
cd poDTest
```

   * **Build the Project**:

```bash
cargo build --release
```

   * **Run Tests**:

```bash
cargo test
```

**Note**: Integration tests require a `Dockerfile` and a health check endpoint (e.g., FastAPI server).

### 3. **Explore the Code**:
   * Source code is in `src/main.rs`.
   * Key dependencies: `tokio`, `reqwest`, `clap`, `ansi_term`, `sysinfo`, `anyhow`.
   * The tool uses flags like `--dockerfile-path` and `--hot-fix` (see README.md).

## How to Contribute

We use GitHub for managing contributions. Follow these steps to contribute:

### 1. **Find or Create an Issue**:
   * Check the Issues tab for open tasks.
   * If your contribution addresses a new bug or feature, create an issue to discuss it first.
   * Include relevant details (e.g., steps to reproduce a bug, proposed feature benefits).

### 2. **Fork and Branch**:
   * Fork the repository to your GitHub account.
   * Create a feature branch:

```bash
git checkout -b feature/your-feature-name
```

Use descriptive branch names (e.g., `fix/health-check-timeout`, `docs/add-examples`).

### 3. **Make Changes**:
   * **Coding Standards**:
      * Follow Rust conventions (use `cargo fmt` for formatting).
      * Write clear, concise code with comments for complex logic.
      * Ensure backward compatibility for existing flags and functionality.
      * Use `ansi_term` for colored logs (yellow: build/testing, green: success, red: failure).
   * **Testing**:
      * Add unit tests in `src/` using Rust's testing framework.
      * Test CLI commands manually:

```bash
cargo run --release -- --dockerfile-path ./Dockerfile --port 8000 --health-check-path /health
```

      * Verify cleanup:

```bash
docker ps -a
docker images | grep my-app
```

   * **Documentation**:
      * Update README.md for new features or flag changes.
      * Add inline comments for significant code changes.

### 4. **Commit Changes**:
   * Write clear commit messages:

```bash
git commit -m "Add support for custom log levels"
```

   * Keep commits focused and atomic.

### 5. **Push and Create a Pull Request**:
   * Push your branch:

```bash
git push origin feature/your-feature-name
```

   * Open a pull request (PR) on the main repository.
   * In the PR description:
      * Reference the related issue (e.g., `Fixes #123`).
      * Summarize changes and their impact.
      * Note any manual testing performed.

### 6. **Code Review**:
   * Respond to feedback from maintainers.
   * Make requested changes and push updates to the same branch.
   * Ensure CI checks (if set up) pass.

## Contribution Types

We welcome various contributions, including:

* **Bug Fixes**: Address issues with the CLI, Docker commands, or health checks.
* **Features**: Add new flags, improve logging, or enhance GitHub Action integration.
* **Documentation**: Improve README.md, add examples, or clarify usage.
* **Tests**: Write unit or integration tests for reliability.
* **GitHub Actions**: Help develop `action.yml` for the GitHub Marketplace.

## Code of Conduct

Please adhere to our Code of Conduct (TBD) to ensure a welcoming and inclusive environment for all contributors.

## Questions?

If you have questions or need help, open an issue or reach out via the GitHub repository. Thank you for contributing to `poDTest`!

*Together, we're building a robust CI/CD tool!*