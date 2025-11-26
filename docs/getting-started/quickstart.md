# Quick Start

Get up and running with rsylla in 5 minutes.

## Prerequisites

- rsylla installed (`pip install rsylla`)
- ScyllaDB or Cassandra running

!!! tip "Start ScyllaDB with Docker"
    ```bash
    docker run --name scylla -d -p 9042:9042 scylladb/scylla
    ```

## Connect to ScyllaDB

=== "Simple Connection"

    ```python
    from rsylla import Session

    # Connect to local ScyllaDB
    session = await Session.connect(["127.0.0.1:9042"])
    print("Connected!")
    ```

=== "With Configuration"

    ```python
    from rsylla import SessionBuilder

    session = await (
        SessionBuilder()
        .known_nodes(["127.0.0.1:9042", "127.0.0.2:9042"])
        .user("username", "password")
        .connection_timeout(5000)
        .pool_size(10)
        .compression("lz4")
        .build()
    )
    ```

## Create a Keyspace and Table

```python
# Create keyspace
await session.execute("""
    CREATE KEYSPACE IF NOT EXISTS quickstart
    WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1}
""")

# Use the keyspace
await session.use_keyspace("quickstart", case_sensitive=False)

# Create table
await session.execute("""
    CREATE TABLE IF NOT EXISTS users (
        id int PRIMARY KEY,
        name text,
        email text
    )
""")
```

## Insert Data

```python
# Insert a single row
await session.execute(
    "INSERT INTO users (id, name, email) VALUES (?, ?, ?)",
    {"id": 1, "name": "Alice", "email": "alice@example.com"}
)
```

## Query Data

```python
# Query all users
result = await session.execute("SELECT * FROM users")

for row in result:
    print(row.columns())  # [1, 'Alice', 'alice@example.com']

# Query with parameters
result = await session.execute(
    "SELECT * FROM users WHERE id = ?",
    {"id": 1}
)

row = result.first_row()
if row:
    print(f"Found: {row[1]}")  # 'Alice'
```

## Use Prepared Statements

For better performance with repeated queries:

```python
# Prepare once
insert_stmt = await session.prepare(
    "INSERT INTO users (id, name, email) VALUES (?, ?, ?)"
)

# Execute many times
for i in range(100):
    await session.execute_prepared(insert_stmt, {
        "id": i,
        "name": f"User {i}",
        "email": f"user{i}@example.com"
    })
```

## Complete Example

Here's a complete working example:

```python
import asyncio
from rsylla import Session

async def main():
    # Connect
    session = await Session.connect(["127.0.0.1:9042"])

    # Setup
    await session.execute("""
        CREATE KEYSPACE IF NOT EXISTS quickstart
        WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1}
    """)
    await session.use_keyspace("quickstart", False)

    await session.execute("""
        CREATE TABLE IF NOT EXISTS users (
            id int PRIMARY KEY,
            name text,
            email text
        )
    """)

    # Insert
    await session.execute(
        "INSERT INTO users (id, name, email) VALUES (?, ?, ?)",
        {"id": 1, "name": "Alice", "email": "alice@example.com"}
    )

    # Query
    result = await session.execute("SELECT * FROM users WHERE id = ?", {"id": 1})
    row = result.first_row()
    if row:
        print(f"User: {row[1]} ({row[2]})")

    # Update
    await session.execute(
        "UPDATE users SET email = ? WHERE id = ?",
        {"email": "alice.new@example.com", "id": 1}
    )

    # Delete
    await session.execute("DELETE FROM users WHERE id = ?", {"id": 1})

    print("Done!")

asyncio.run(main())
```

## What's Next?

Now that you've got the basics, explore:

<div class="grid cards" markdown>

-   :material-school:{ .lg .middle } **Tutorial**

    ---

    In-depth walkthrough with more examples

    [:octicons-arrow-right-24: Full Tutorial](tutorial.md)

-   :material-database:{ .lg .middle } **Data Types**

    ---

    Working with CQL data types in Python

    [:octicons-arrow-right-24: Data Types](../guide/data-types.md)

-   :material-rocket-launch:{ .lg .middle } **Prepared Statements**

    ---

    Optimize performance with prepared statements

    [:octicons-arrow-right-24: Examples](../examples/prepared-statements.md)

-   :material-layers:{ .lg .middle } **Batch Operations**

    ---

    Execute multiple statements atomically

    [:octicons-arrow-right-24: Batch Ops](../examples/batch-operations.md)

</div>
