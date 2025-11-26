# rsylla

<div align="center">
<h2>The fastest Python driver for ScyllaDB</h2>
<p>High-performance Python bindings using the official <a href="https://github.com/scylladb/scylla-rust-driver">scylla-rust-driver</a></p>
</div>

---

## Why rsylla?

**rsylla** delivers exceptional performance for Python applications connecting to ScyllaDB and Cassandra:

<div class="grid cards" markdown>

-   :material-speedometer:{ .lg .middle } **~3.9x Faster**

    ---

    Outperforms the DataStax cassandra-driver by nearly 4x in read/write operations

-   :material-lightning-bolt:{ .lg .middle } **85,000+ ops/sec**

    ---

    Achieve over 85,000 operations per second for read and write workloads

-   :material-timer-sand:{ .lg .middle } **Sub-millisecond Latency**

    ---

    Average latencies of 0.37-0.43ms for prepared statement operations

-   :material-memory:{ .lg .middle } **Zero-Copy Design**

    ---

    Efficient data handling with minimal memory overhead between Rust and Python

</div>

## Performance Comparison

Based on comprehensive benchmarks with 32 concurrent clients:

| Operation | rsylla | acsylla | cassandra-driver |
|-----------|--------|---------|------------------|
| **Read (prepared)** | 85,920 ops/s | 71,450 ops/s | 22,160 ops/s |
| **Write (prepared)** | 81,260 ops/s | 66,720 ops/s | 20,340 ops/s |
| **Latency (avg)** | 0.37-0.43 ms | 0.45-0.54 ms | 1.44-1.67 ms |

[See detailed benchmarks :material-arrow-right:](performance/benchmarks.md){ .md-button }

## Quick Example

```python
import asyncio
from rsylla import Session

async def main():
    # Connect to ScyllaDB cluster
    session = await Session.connect(["127.0.0.1:9042"])

    # Execute a query
    result = await session.execute(
        "SELECT * FROM users WHERE id = ?",
        {"id": 123}
    )

    # Iterate over rows
    for row in result:
        print(row.columns())

asyncio.run(main())
```

## Features

<div class="grid cards" markdown>

-   :material-rocket-launch:{ .lg .middle } **Native Rust Performance**

    ---

    Built on top of the official Rust driver for ScyllaDB, ensuring maximum throughput and reliability.

-   :material-api:{ .lg .middle } **Full API Coverage**

    ---

    Every function and method from the Rust driver has a Python equivalent with comprehensive type hints.

-   :material-sync:{ .lg .middle } **Async Runtime**

    ---

    Efficient async operations powered by Tokio, integrating seamlessly with Python's asyncio.

-   :material-tools:{ .lg .middle } **Easy to Use**

    ---

    Pythonic API that feels natural, with extensive documentation and examples.

</div>

## Installation

=== "pip"

    ```bash
    pip install rsylla
    ```

=== "From Source"

    ```bash
    # Install Rust and maturin
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    pip install maturin

    # Clone and build
    git clone https://github.com/r4fek/rsylla.git
    cd rsylla
    maturin develop --release
    ```

[Get Started :material-arrow-right:](getting-started/installation.md){ .md-button .md-button--primary }

## Core Components

| Component | Description |
|-----------|-------------|
| [**Session**](api/session.md) | Main entry point for database operations |
| [**SessionBuilder**](api/session.md#sessionbuilder) | Fluent builder for session configuration |
| [**Query**](api/query.md) | Configurable query with consistency levels and options |
| [**PreparedStatement**](api/query.md#preparedstatement) | Pre-compiled statements for optimal performance |
| [**Batch**](api/batch.md) | Atomic batch operations |
| [**QueryResult**](api/results.md) | Result set with iteration and type conversion |

## What's Next?

<div class="grid cards" markdown>

-   :material-book-open-variant:{ .lg .middle } **Getting Started**

    ---

    Follow our step-by-step tutorial to build your first application

    [:octicons-arrow-right-24: Tutorial](getting-started/tutorial.md)

-   :material-code-braces:{ .lg .middle } **API Reference**

    ---

    Complete documentation for all classes and methods

    [:octicons-arrow-right-24: API Docs](api/overview.md)

-   :material-lightbulb:{ .lg .middle } **Examples**

    ---

    Real-world code examples and patterns

    [:octicons-arrow-right-24: Examples](examples/basic.md)

-   :material-chart-line:{ .lg .middle } **Best Practices**

    ---

    Production-ready patterns and optimizations

    [:octicons-arrow-right-24: Best Practices](guide/best-practices.md)

</div>

## License

rsylla is dual-licensed under MIT or Apache-2.0, matching the scylla-rust-driver license.

## Links

- [GitHub Repository](https://github.com/r4fek/rsylla)
- [PyPI Package](https://pypi.org/project/rsylla/)
- [ScyllaDB Documentation](https://docs.scylladb.com/)
- [scylla-rust-driver](https://github.com/scylladb/scylla-rust-driver)
