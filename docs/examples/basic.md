# Basic Usage Examples

This page demonstrates fundamental rsylla operations.

## Connecting to ScyllaDB

### Simple Connection

```python
import asyncio
from rsylla import Session

async def main():
    # Connect to a single node
    session = await Session.connect(["127.0.0.1:9042"])
    print("Connected!")

asyncio.run(main())
```

### Multi-Node Connection

```python
from rsylla import SessionBuilder

async def main():
    session = await (
        SessionBuilder()
        .known_nodes([
            "node1.cluster.local:9042",
            "node2.cluster.local:9042",
            "node3.cluster.local:9042"
        ])
        .pool_size(10)
        .compression("lz4")
        .build()
    )
```

### Authenticated Connection

```python
session = await (
    SessionBuilder()
    .known_nodes(["127.0.0.1:9042"])
    .user("myuser", "mypassword")
    .build()
)
```

## Creating Keyspaces and Tables

```python
# Create keyspace
await session.execute("""
    CREATE KEYSPACE IF NOT EXISTS myapp
    WITH replication = {
        'class': 'SimpleStrategy',
        'replication_factor': 1
    }
""")

# Switch to keyspace
await session.use_keyspace("myapp", case_sensitive=False)

# Create table
await session.execute("""
    CREATE TABLE IF NOT EXISTS users (
        id int PRIMARY KEY,
        name text,
        email text,
        created_at timestamp
    )
""")

# Wait for schema to sync
await session.await_schema_agreement()
```

## CRUD Operations

### Create (Insert)

```python
import time

# Insert a row
await session.execute(
    "INSERT INTO users (id, name, email, created_at) VALUES (?, ?, ?, ?)",
    {
        "id": 1,
        "name": "Alice",
        "email": "alice@example.com",
        "created_at": int(time.time() * 1000)
    }
)
```

### Read (Select)

```python
# Select all
result = await session.execute("SELECT * FROM users")
for row in result:
    print(row.columns())

# Select with condition
result = await session.execute(
    "SELECT * FROM users WHERE id = ?",
    {"id": 1}
)
row = result.first_row()
if row:
    id, name, email, created_at = row.columns()
    print(f"User: {name} ({email})")
```

### Update

```python
await session.execute(
    "UPDATE users SET email = ? WHERE id = ?",
    {"email": "alice.new@example.com", "id": 1}
)
```

### Delete

```python
await session.execute(
    "DELETE FROM users WHERE id = ?",
    {"id": 1}
)
```

## Working with Results

```python
result = await session.execute("SELECT * FROM users")

# Check if empty
if not result:
    print("No users found")

# Get count
print(f"Found {len(result)} users")

# Iterate
for row in result:
    print(row[0], row[1], row[2])

# Get first row
first = result.first_row()

# Get all as list
all_rows = result.rows()

# Get as dictionaries
dicts = result.rows_typed()
```

## Using Query Options

```python
from rsylla import Query

# Create query with options
query = (
    Query("SELECT * FROM users")
    .with_consistency("QUORUM")
    .with_page_size(100)
    .with_timeout(5000)
)

result = await session.query(query)
```

## Error Handling

```python
from rsylla import ScyllaError

try:
    result = await session.execute("SELECT * FROM nonexistent")
except ScyllaError as e:
    print(f"Query failed: {e}")
```

## Complete Example

```python
import asyncio
import time
from rsylla import Session, ScyllaError

async def main():
    # Connect
    session = await Session.connect(["127.0.0.1:9042"])

    try:
        # Setup
        await session.execute("""
            CREATE KEYSPACE IF NOT EXISTS example
            WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1}
        """)
        await session.use_keyspace("example", False)

        await session.execute("""
            CREATE TABLE IF NOT EXISTS users (
                id int PRIMARY KEY,
                name text,
                email text
            )
        """)

        # Insert
        users = [
            (1, "Alice", "alice@example.com"),
            (2, "Bob", "bob@example.com"),
            (3, "Charlie", "charlie@example.com"),
        ]

        for id, name, email in users:
            await session.execute(
                "INSERT INTO users (id, name, email) VALUES (?, ?, ?)",
                {"id": id, "name": name, "email": email}
            )

        # Query
        result = await session.execute("SELECT * FROM users")
        print(f"\nAll users ({len(result)}):")
        for row in result:
            print(f"  {row[0]}: {row[1]} <{row[2]}>")

        # Update
        await session.execute(
            "UPDATE users SET email = ? WHERE id = ?",
            {"email": "alice.updated@example.com", "id": 1}
        )

        # Verify
        result = await session.execute(
            "SELECT name, email FROM users WHERE id = ?",
            {"id": 1}
        )
        row = result.first_row()
        print(f"\nUpdated: {row[0]} -> {row[1]}")

        # Delete
        await session.execute("DELETE FROM users WHERE id = ?", {"id": 3})

        # Final count
        result = await session.execute("SELECT COUNT(*) FROM users")
        print(f"\nFinal count: {result.first_row()[0]}")

    except ScyllaError as e:
        print(f"Error: {e}")

asyncio.run(main())
```
