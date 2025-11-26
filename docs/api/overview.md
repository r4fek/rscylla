# API Overview

rsylla provides Python bindings for the ScyllaDB Rust driver. This section documents all public classes, methods, and types.

## Core Classes

| Class | Description |
|-------|-------------|
| [`Session`](session.md#session) | Main entry point for database operations |
| [`SessionBuilder`](session.md#sessionbuilder) | Fluent builder for session configuration |
| [`Query`](query.md#query) | Configurable query with execution options |
| [`PreparedStatement`](query.md#preparedstatement) | Pre-compiled statement for optimal performance |
| [`Batch`](batch.md) | Batch operations for multiple statements |
| [`QueryResult`](results.md#queryresult) | Result set from query execution |
| [`Row`](results.md#row) | Single row from a result set |
| [`ScyllaError`](errors.md) | Exception for database errors |

## Quick Reference

### Connecting

```python
from rsylla import Session, SessionBuilder

# Simple connection
session = await Session.connect(["127.0.0.1:9042"])

# Advanced connection
session = await (
    SessionBuilder()
    .known_nodes(["node1:9042", "node2:9042"])
    .user("username", "password")
    .compression("lz4")
    .pool_size(10)
    .build()
)
```

### Executing Queries

```python
# Simple query
result = await session.execute("SELECT * FROM users")

# Query with parameters
result = await session.execute(
    "SELECT * FROM users WHERE id = ?",
    {"id": 123}
)

# Query with options
from rsylla import Query

query = (
    Query("SELECT * FROM users")
    .with_consistency("QUORUM")
    .with_page_size(1000)
)
result = await session.query(query)
```

### Prepared Statements

```python
# Prepare
prepared = await session.prepare(
    "INSERT INTO users (id, name) VALUES (?, ?)"
)

# Execute
await session.execute_prepared(prepared, {"id": 1, "name": "Alice"})
```

### Batch Operations

```python
from rsylla import Batch

batch = Batch("logged")
batch.append_statement("INSERT INTO users (id, name) VALUES (?, ?)")
batch.append_statement("INSERT INTO users (id, name) VALUES (?, ?)")

await session.batch(batch, [
    {"id": 1, "name": "Alice"},
    {"id": 2, "name": "Bob"}
])
```

### Working with Results

```python
result = await session.execute("SELECT * FROM users")

# Iterate
for row in result:
    print(row.columns())

# First row
row = result.first_row()

# Single row (raises if not exactly one)
row = result.single_row()

# All rows
rows = result.rows()

# Rows as dictionaries
rows_dict = result.rows_typed()

# Length
print(len(result))

# Boolean check
if result:
    print("Has rows")
```

## Consistency Levels

rsylla supports all CQL consistency levels:

| Level | Description |
|-------|-------------|
| `ANY` | Write acknowledged by any node |
| `ONE` | Read/write from one replica |
| `TWO` | Read/write from two replicas |
| `THREE` | Read/write from three replicas |
| `QUORUM` | Majority of replicas |
| `ALL` | All replicas |
| `LOCAL_QUORUM` | Majority in local datacenter |
| `EACH_QUORUM` | Majority in each datacenter |
| `LOCAL_ONE` | One replica in local datacenter |

Serial consistency levels (for LWT):

| Level | Description |
|-------|-------------|
| `SERIAL` | Serial read across all datacenters |
| `LOCAL_SERIAL` | Serial read in local datacenter |

## Type Mapping

Python types are automatically converted to CQL types:

| Python | CQL |
|--------|-----|
| `bool` | `boolean` |
| `int` | `int`, `bigint` |
| `float` | `float`, `double` |
| `str` | `text`, `varchar` |
| `bytes` | `blob` |
| `list` | `list`, `set` |
| `dict` | `map` |
| `None` | `NULL` |

See [Data Types Guide](../guide/data-types.md) for detailed information.

## Error Handling

All rsylla operations can raise `ScyllaError`:

```python
from rsylla import ScyllaError

try:
    result = await session.execute("INVALID QUERY")
except ScyllaError as e:
    print(f"Error: {e}")
```

See [Errors](errors.md) for more details.

## Async/Await

All rsylla operations are asynchronous. Always use `await`:

```python
import asyncio

async def main():
    session = await Session.connect(["127.0.0.1:9042"])
    result = await session.execute("SELECT * FROM users")
    # ...

asyncio.run(main())
```

## Module Exports

```python
from rsylla import (
    Session,
    SessionBuilder,
    Query,
    PreparedStatement,
    Batch,
    QueryResult,
    Row,
    ScyllaError
)
```
