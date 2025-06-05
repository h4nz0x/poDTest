# poDTest

poDTest is a Rust-based CLI utility for building and testing Docker images in CI/CD pipelines. It automates the process of building a Docker image, running a container, performing health checks, and cleaning up resources. With configurable flags, it supports custom Dockerfiles, health check endpoints, and urgent deployment scenarios via a hot-fix mode. Designed for integration as a GitHub Action, this tool ensures reliable Docker image validation for your projects.

## Features

- **Flexible Dockerfile Selection**: Specify custom Dockerfiles (e.g., Dockerfile.staging, Dockerfile.prod) using `--dockerfile-path` (default: Dockerfile).
- **Health Check Validation**: Test container health via a configurable endpoint (e.g., /health, default: /up) with retries and timeouts.
- **Port Mapping**: Map an external port (default: 3000) to your container's internal port (e.g., `--port=8000`).
- **Hot-Fix Mode**: Skip health checks for rapid deployment using `--hot-fix`.
- **Build Timeout**: Set a maximum build duration with `--build-timeout` (default: 1200 seconds).
- **Customizable Health Checks**: Configure retry attempts (`--health-check-interval`, default: 5) and timeout per attempt (`--health-check-timeout`, default: 5 seconds).
- **Clean Logging**: Color-coded logs (yellow for build/testing, green for success, red for failure) with clear headers for each step.
- **Resource Cleanup**: Automatically stops and removes containers and images after testing.
- **GitHub Actions Ready**: Built for seamless integration into CI/CD workflows.

## Prerequisites

- **Rust**: Version 1.65 or higher (install via [rustup](https://rustup.rs/)).
- **Docker**: Installed and running with user permissions (see [Docker installation](https://docs.docker.com/get-docker/)).
- **Git**: For cloning the repository.
- **Optional**: A Dockerfile and application (e.g., FastAPI server) to test.

## Installation

### Clone the Repository:
```bash
git clone https://github.com/h4nz0x/poDTest.git
cd poDTest
```

### Build the Tool:
```bash
cargo build --release
```

The binary will be located at `target/release/poDTest`.

### Verify Installation:
```bash
./target/release/poDTest --help
```

This displays available flags and their descriptions.

## Usage

Run the tool with `cargo run --release -- [flags]` or use the compiled binary (`./target/release/poDTest [flags]`). Below are the supported flags:

| Flag | Description | Default |
|------|-------------|---------|
| `--dockerfile-path` | Path to the Dockerfile (e.g., Dockerfile.prod) | Dockerfile |
| `--hot-fix` | Skip health checks for urgent deployments | false |
| `--build-timeout` | Maximum build time in seconds | 1200 |
| `--health-check-path` | Health check endpoint (e.g., /health) | /up |
| `--port` | Internal container port (maps to external 3000) | 80 |
| `--health-check-timeout` | Timeout per health check attempt in seconds | 5 |
| `--health-check-interval` | Number of health check retries | 5 |

## Examples

### Test a Docker Image:
```bash
cargo run --release -- --dockerfile-path ./Dockerfile --port 8000 --health-check-path /health
```

Builds the image, runs a container (mapping 3000:8000), tests /health, and cleans up.

### Hot-Fix Deployment:
```bash
cargo run --release -- --dockerfile-path Dockerfile.prod --hot-fix
```

Builds the image without testing or cleanup for rapid deployment.

### Custom Health Check:
```bash
cargo run --release -- --dockerfile-path Dockerfile.staging --port 8080 --health-check-path /check --health-check-timeout 10 --health-check-interval 3
```

Tests /check with 3 retries, 10-second timeout per attempt.

## GitHub Actions Integration

This tool is designed to be used as a GitHub Action. A dedicated `action.yml` is under development for the GitHub Marketplace. To use it locally in a workflow:

### Add the Binary:
Copy `target/release/poDTest` to your repository or build it in your workflow.

### Example Workflow (placeholder):
```yaml
name: Test Docker Image
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Build Container Test Tool
        run: cargo build --release
      - name: Test Docker Image
        run: ./target/release/poDTest --dockerfile-path Dockerfile --port 8000 --health-check-path /health
```

**Stay tuned for the official GitHub Action release on the GitHub Marketplace!**

## Testing

To verify the tool's functionality:

### Prepare a Dockerfile:
Ensure your Dockerfile builds an application with a health check endpoint (e.g., FastAPI server with /health).

Example `main.py`:
```python
from fastapi import FastAPI
app = FastAPI()

@app.get("/health")
async def health():
    return {"status": "ok"}
```

### Run Tests:

**Success Case:**
```bash
cargo run --release -- --dockerfile-path /path/to/Dockerfile --port 8000 --health-check-path /health
```
Expect green success messages and cleanup.

**Failure Case:**
```bash
cargo run --release -- --dockerfile-path /path/to/Dockerfile --port 8000 --health-check-path /invalid
```
Expect red failure messages with logs.

**Hot-Fix Case:**
```bash
cargo run --release -- --dockerfile-path /path/to/Dockerfile --hot-fix
```
Expect build only, no testing.

### Verify Cleanup:
```bash
docker ps -a
docker images | grep my-app
```

No containers or images named `my-app` should remain.

## Contributing

Contributions are welcome! To contribute:

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/your-feature`).
3. Commit changes (`git commit -m "Add your feature"`).
4. Push to the branch (`git push origin feature/your-feature`).
5. Open a pull request.

Please include tests and update documentation as needed. See `CONTRIBUTING.md` for details.

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.

## Contact

For issues or feature requests, open an issue on the GitHub repository or contact the maintainers.

---

## poDTest