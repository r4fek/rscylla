# Installation

This guide covers different ways to install rsylla.

## Requirements

- **Python 3.11+** (compiled with abi3 for Python 3.11+)
- **ScyllaDB or Cassandra** (for testing)

## Install from PyPI

The simplest way to install rsylla is using pip:

```bash
pip install rsylla
```

This installs a pre-built wheel for your platform.

## Verify Installation

After installation, verify rsylla is working:

```python
import rsylla

# Check available exports
print(dir(rsylla))
# ['Batch', 'PreparedStatement', 'Query', 'QueryResult', 'Row', 'ScyllaError', 'Session', 'SessionBuilder']

# Import main classes
from rsylla import Session, SessionBuilder, Query, Batch
print("rsylla installed successfully!")
```

## Build from Source

Building from source requires Rust and maturin.

### Prerequisites

#### 1. Install Rust

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

!!! note "Windows Users"
    On Windows, download and run the installer from [rustup.rs](https://rustup.rs/).

#### 2. Install maturin

```bash
pip install maturin
```

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/r4fek/rsylla.git
cd rsylla

# Development build (includes debug symbols)
maturin develop

# Release build (optimized)
maturin develop --release
```

### Build Wheel

To create a distributable wheel:

```bash
# Build wheel
maturin build --release

# Install the wheel
pip install target/wheels/*.whl
```

### Using Make

If you have `make` available:

```bash
# Development install
make develop

# Production build
make build

# Run tests
make test

# Format code
make fmt
```

## Virtual Environment Setup

It's recommended to use a virtual environment:

```bash
# Create virtual environment
python -m venv .venv

# Activate (Linux/macOS)
source .venv/bin/activate

# Activate (Windows)
.venv\Scripts\activate

# Install rsylla
pip install rsylla
```

## Docker Development

For development with Docker:

```dockerfile
FROM python:3.11-slim

# Install Rust
RUN apt-get update && apt-get install -y curl build-essential
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install maturin
RUN pip install maturin

# Copy and build
WORKDIR /app
COPY . .
RUN maturin develop --release
```

## Setting Up ScyllaDB

### Using Docker (Recommended)

```bash
# Start ScyllaDB
docker run --name scylla -d -p 9042:9042 scylladb/scylla

# Wait for it to be ready (about 30 seconds)
docker logs -f scylla

# Verify it's running
docker exec -it scylla nodetool status
```

### Using Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3'
services:
  scylla:
    image: scylladb/scylla
    ports:
      - "9042:9042"
    volumes:
      - scylla-data:/var/lib/scylla
    command: --smp 1 --memory 512M

volumes:
  scylla-data:
```

Then run:

```bash
docker-compose up -d
```

### Connect with cqlsh

Verify ScyllaDB is working:

```bash
docker exec -it scylla cqlsh

# In cqlsh:
cqlsh> SELECT cluster_name, release_version FROM system.local;
```

## Troubleshooting

### ImportError: No module named '_rsylla'

This typically means:

1. **rsylla not installed properly** - Run `pip install rsylla` again
2. **Wrong Python environment** - Ensure you're using the correct virtual environment
3. **Build issue** - If building from source, run `maturin develop --release`

### Rust Build Errors

```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
maturin develop --release
```

### Connection Refused to ScyllaDB

1. Verify ScyllaDB is running:
   ```bash
   docker ps | grep scylla
   ```

2. Check the port is accessible:
   ```bash
   netstat -an | grep 9042
   ```

3. Test with cqlsh:
   ```bash
   docker exec -it scylla cqlsh
   ```

### macOS ARM (M1/M2) Issues

If you encounter issues on Apple Silicon:

```bash
# Ensure Rust is installed for ARM
rustup target add aarch64-apple-darwin

# Rebuild
cargo clean
maturin develop --release
```

## Next Steps

Now that rsylla is installed, continue to:

- [Quick Start](quickstart.md) - Connect and run your first query
- [Tutorial](tutorial.md) - Build a complete application
- [Examples](../examples/basic.md) - See more code examples
