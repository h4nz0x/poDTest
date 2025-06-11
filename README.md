# poDTest

A GitHub Action to run Docker container tests using the `poDTest` Rust CLI tool. This action builds, tests, and cleans up a Docker web app container based on a specified Dockerfile, with support for health checks and hot-fix mode for GitHub Actions integration.

## Features

* Builds and tests Docker web app containers.
* Supports custom ports (e.g., maps `3000:8000`).
* Performs health checks (e.g., `http://localhost:3000/health`).
* Cleans up Docker resources after testing.
* Supports `--hot-fix true` to skip execution and mark tests as successful.
* Color-coded output: yellow for titles/warnings, white for logs, red for errors, green for success.

## Prerequisites

* Docker installed and running on the GitHub Actions runner.
* A Dockerfile in the specified path (e.g., `./Dockerfile`).
* A service responding to the health check endpoint (e.g., `/health`).

## Usage

Add the following to your GitHub Actions workflow (e.g., `.github/workflows/test.yml`):

```yaml
name: Test Docker Container
on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v4.1.2
      - name: Run poDTest
        uses: h4nz0x/poDTest@v4
        with:
          dockerfile-path: ./deployments
          port: 8000
          health-check-path: /health
          health-check-timeout: 10
          health-check-interval: 5
          build-timeout: 300
          hot-fix: false
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `dockerfile-path` | Path to the Dockerfile | No | `./deployments` |
| `port` | Internal port to expose | No | `8000` |
| `health-check-path` | Health check endpoint path | No | `/health` |
| `health-check-timeout` | Timeout for health check in seconds | No | `10` |
| `health-check-interval` | Number of health check attempts | No | `5` |
| `build-timeout` | Timeout for image build in seconds | No | `300` |
| `hot-fix` | Enable hot-fix mode (true/false) | No | `""` |

## Example with Hot-Fix

To skip execution (e.g., for GitHub Actions without Docker):

```yaml
- name: Run poDTest with Hot-Fix
  uses: h4nz0x/poDTest@v4.1.2
  with:
    hot-fix: true
```

## License

MIT