# Session API

The Session class is the main entry point for interacting with ScyllaDB. It manages connections, executes queries, and handles cluster operations.

## SessionBuilder

Use `SessionBuilder` to create sessions with custom configuration.

### Constructor

```python
from rsylla import SessionBuilder

builder = SessionBuilder()
```

### Methods

#### `known_node(hostname: str) -> SessionBuilder`

Add a single known node.

```python
builder = SessionBuilder().known_node("127.0.0.1:9042")
```

**Parameters:**

- `hostname` - Node address in format "host:port"

**Returns:** Self for method chaining

---

#### `known_nodes(hostnames: List[str]) -> SessionBuilder`

Add multiple known nodes.

```python
builder = SessionBuilder().known_nodes([
    "node1.example.com:9042",
    "node2.example.com:9042",
    "node3.example.com:9042"
])
```

**Parameters:**

- `hostnames` - List of node addresses

**Returns:** Self for method chaining

---

#### `use_keyspace(keyspace_name: str, case_sensitive: bool) -> SessionBuilder`

Set the default keyspace for the session.

```python
builder = SessionBuilder().use_keyspace("my_keyspace", case_sensitive=False)
```

**Parameters:**

- `keyspace_name` - Name of the keyspace
- `case_sensitive` - Whether the keyspace name is case-sensitive

**Returns:** Self for method chaining

---

#### `connection_timeout(duration_ms: int) -> SessionBuilder`

Set the connection timeout.

```python
builder = SessionBuilder().connection_timeout(10000)  # 10 seconds
```

**Parameters:**

- `duration_ms` - Timeout in milliseconds

**Returns:** Self for method chaining

---

#### `pool_size(size: int) -> SessionBuilder`

Set the connection pool size per host.

```python
builder = SessionBuilder().pool_size(20)
```

**Parameters:**

- `size` - Number of connections per host (must be > 0)

**Raises:** `ValueError` if size is 0

**Returns:** Self for method chaining

---

#### `user(username: str, password: str) -> SessionBuilder`

Set authentication credentials.

```python
builder = SessionBuilder().user("myuser", "mypassword")
```

**Parameters:**

- `username` - Authentication username
- `password` - Authentication password

**Returns:** Self for method chaining

---

#### `compression(compression: Optional[str]) -> SessionBuilder`

Set compression type for network traffic.

```python
builder = SessionBuilder().compression("lz4")
```

**Parameters:**

- `compression` - One of: `"lz4"`, `"snappy"`, or `None`

**Raises:** `ValueError` for invalid compression type

**Returns:** Self for method chaining

---

#### `tcp_nodelay(nodelay: bool) -> SessionBuilder`

Enable or disable TCP_NODELAY (Nagle's algorithm).

```python
builder = SessionBuilder().tcp_nodelay(True)
```

**Parameters:**

- `nodelay` - `True` to disable Nagle's algorithm (lower latency)

**Returns:** Self for method chaining

---

#### `tcp_keepalive(keepalive_ms: Optional[int]) -> SessionBuilder`

Configure TCP keepalive.

!!! note
    TCP keepalive configuration is handled at the OS level in recent versions.

```python
builder = SessionBuilder().tcp_keepalive(60000)  # 60 seconds
```

**Parameters:**

- `keepalive_ms` - Keepalive interval in milliseconds, or `None` to disable

**Returns:** Self for method chaining

---

#### `async build() -> Session`

Build and connect the session.

```python
session = await SessionBuilder().known_nodes(["127.0.0.1:9042"]).build()
```

**Returns:** Connected `Session` instance

**Raises:** `ScyllaError` on connection failure

---

### Complete Example

```python
from rsylla import SessionBuilder

session = await (
    SessionBuilder()
    .known_nodes(["node1:9042", "node2:9042", "node3:9042"])
    .user("app_user", "secure_password")
    .use_keyspace("production", case_sensitive=False)
    .pool_size(20)
    .connection_timeout(10000)
    .compression("lz4")
    .tcp_nodelay(True)
    .build()
)
```

---

## Session

The `Session` class represents an active connection to the ScyllaDB cluster.

### Static Methods

#### `async connect(nodes: List[str]) -> Session`

Create a session with default configuration.

```python
from rsylla import Session

session = await Session.connect(["127.0.0.1:9042"])
```

**Parameters:**

- `nodes` - List of node addresses

**Returns:** Connected `Session` instance

**Raises:** `ScyllaError` on connection failure

---

### Instance Methods

#### `async execute(query: str, values: Optional[Dict[str, Any]] = None) -> QueryResult`

Execute a CQL query.

```python
# Simple query
result = await session.execute("SELECT * FROM users")

# Query with parameters
result = await session.execute(
    "SELECT * FROM users WHERE id = ?",
    {"id": 123}
)
```

**Parameters:**

- `query` - CQL query string
- `values` - Optional dictionary of parameter values

**Returns:** `QueryResult` containing the results

**Raises:** `ScyllaError` on query failure

---

#### `async query(query: Query, values: Optional[Dict[str, Any]] = None) -> QueryResult`

Execute a Query object with configuration.

```python
from rsylla import Query

query = (
    Query("SELECT * FROM users WHERE id = ?")
    .with_consistency("QUORUM")
    .with_page_size(1000)
)

result = await session.query(query, {"id": 123})
```

**Parameters:**

- `query` - Configured `Query` object
- `values` - Optional dictionary of parameter values

**Returns:** `QueryResult` containing the results

**Raises:** `ScyllaError` on query failure

---

#### `async prepare(query: str) -> PreparedStatement`

Prepare a statement for repeated execution.

```python
prepared = await session.prepare(
    "INSERT INTO users (id, name, email) VALUES (?, ?, ?)"
)
```

**Parameters:**

- `query` - CQL query string to prepare

**Returns:** `PreparedStatement` ready for execution

**Raises:** `ScyllaError` on preparation failure

---

#### `async execute_prepared(prepared: PreparedStatement, values: Optional[Dict[str, Any]] = None) -> QueryResult`

Execute a prepared statement.

```python
prepared = await session.prepare("SELECT * FROM users WHERE id = ?")

result = await session.execute_prepared(prepared, {"id": 123})
```

**Parameters:**

- `prepared` - `PreparedStatement` to execute
- `values` - Optional dictionary of parameter values

**Returns:** `QueryResult` containing the results

**Raises:** `ScyllaError` on execution failure

---

#### `async batch(batch: Batch, values: List[Dict[str, Any]]) -> QueryResult`

Execute a batch of statements.

```python
from rsylla import Batch

batch = Batch("logged")
batch.append_statement("INSERT INTO users (id, name) VALUES (?, ?)")
batch.append_statement("INSERT INTO users (id, name) VALUES (?, ?)")

result = await session.batch(batch, [
    {"id": 1, "name": "Alice"},
    {"id": 2, "name": "Bob"}
])
```

**Parameters:**

- `batch` - `Batch` object containing statements
- `values` - List of dictionaries, one per statement

**Returns:** `QueryResult` (usually empty for write operations)

**Raises:** `ScyllaError` on batch failure

---

#### `async use_keyspace(keyspace_name: str, case_sensitive: bool) -> None`

Switch to a different keyspace.

```python
await session.use_keyspace("production", case_sensitive=False)
```

**Parameters:**

- `keyspace_name` - Name of the keyspace
- `case_sensitive` - Whether the name is case-sensitive

**Raises:** `ScyllaError` if keyspace doesn't exist

---

#### `async await_schema_agreement() -> bool`

Wait for schema to synchronize across the cluster.

```python
# After creating a table
await session.execute("CREATE TABLE ...")

if await session.await_schema_agreement():
    print("Schema synchronized!")
```

**Returns:** `True` when schema agreement is reached

**Raises:** `ScyllaError` on timeout or failure

---

#### `get_cluster_data() -> str`

Get cluster metadata information.

```python
info = session.get_cluster_data()
print(info)
```

**Returns:** String representation of cluster data

---

#### `get_keyspace() -> Optional[str]`

Get the current keyspace.

```python
keyspace = session.get_keyspace()
if keyspace:
    print(f"Current keyspace: {keyspace}")
```

**Returns:** Current keyspace name or `None`

---

### Usage Examples

#### Basic CRUD Operations

```python
from rsylla import Session

async def crud_example():
    session = await Session.connect(["127.0.0.1:9042"])
    await session.use_keyspace("test", False)

    # Create
    await session.execute(
        "INSERT INTO users (id, name, email) VALUES (?, ?, ?)",
        {"id": 1, "name": "Alice", "email": "alice@example.com"}
    )

    # Read
    result = await session.execute(
        "SELECT * FROM users WHERE id = ?",
        {"id": 1}
    )
    row = result.first_row()
    if row:
        print(row.columns())

    # Update
    await session.execute(
        "UPDATE users SET email = ? WHERE id = ?",
        {"email": "alice.new@example.com", "id": 1}
    )

    # Delete
    await session.execute(
        "DELETE FROM users WHERE id = ?",
        {"id": 1}
    )
```

#### Connection Pool Pattern

```python
class Database:
    """Singleton database connection"""

    _session = None

    @classmethod
    async def get_session(cls):
        if cls._session is None:
            cls._session = await (
                SessionBuilder()
                .known_nodes(["node1:9042", "node2:9042"])
                .pool_size(20)
                .compression("lz4")
                .build()
            )
        return cls._session

# Usage
session = await Database.get_session()
result = await session.execute("SELECT * FROM users")
```

#### Retry Pattern

```python
import asyncio
from rsylla import Session, ScyllaError

async def execute_with_retry(session, query, values=None, max_retries=3):
    """Execute query with exponential backoff retry"""
    for attempt in range(max_retries):
        try:
            return await session.execute(query, values)
        except ScyllaError as e:
            if attempt == max_retries - 1:
                raise
            wait_time = 2 ** attempt
            print(f"Retry {attempt + 1}/{max_retries} after {wait_time}s: {e}")
            await asyncio.sleep(wait_time)
```
