# Query API

The Query and PreparedStatement classes allow you to configure query execution options like consistency levels, timeouts, and paging.

## Query

The `Query` class represents a CQL query with configurable execution options.

### Constructor

```python
from rsylla import Query

query = Query("SELECT * FROM users WHERE id = ?")
```

**Parameters:**

- `query` - CQL query string

### Methods

#### `with_consistency(consistency: str) -> Query`

Set the consistency level for this query.

```python
query = Query("SELECT * FROM users").with_consistency("QUORUM")
```

**Parameters:**

- `consistency` - One of: `ANY`, `ONE`, `TWO`, `THREE`, `QUORUM`, `ALL`, `LOCAL_QUORUM`, `EACH_QUORUM`, `LOCAL_ONE`

**Raises:** `ValueError` for invalid consistency level

**Returns:** Self for method chaining

---

#### `with_serial_consistency(serial_consistency: str) -> Query`

Set the serial consistency level (for lightweight transactions).

```python
query = (
    Query("UPDATE users SET email = ? WHERE id = ? IF EXISTS")
    .with_serial_consistency("SERIAL")
)
```

**Parameters:**

- `serial_consistency` - One of: `SERIAL`, `LOCAL_SERIAL`

**Raises:** `ValueError` for invalid serial consistency

**Returns:** Self for method chaining

---

#### `with_page_size(page_size: int) -> Query`

Set the page size for result pagination.

```python
query = Query("SELECT * FROM large_table").with_page_size(1000)
```

**Parameters:**

- `page_size` - Number of rows per page

**Returns:** Self for method chaining

---

#### `with_timestamp(timestamp: int) -> Query`

Set a specific timestamp for the query.

```python
query = (
    Query("INSERT INTO users (id, name) VALUES (?, ?)")
    .with_timestamp(1234567890000)  # Microseconds
)
```

**Parameters:**

- `timestamp` - Timestamp in microseconds

**Returns:** Self for method chaining

---

#### `with_timeout(timeout_ms: int) -> Query`

Set a timeout for the query.

```python
query = Query("SELECT * FROM users").with_timeout(5000)  # 5 seconds
```

**Parameters:**

- `timeout_ms` - Timeout in milliseconds

**Returns:** Self for method chaining

---

#### `with_tracing(tracing: bool) -> Query`

Enable or disable query tracing.

```python
query = Query("SELECT * FROM users").with_tracing(True)
```

**Parameters:**

- `tracing` - `True` to enable tracing

**Returns:** Self for method chaining

!!! warning "Performance Impact"
    Tracing has overhead. Only enable for debugging.

---

#### `is_idempotent() -> bool`

Check if the query is marked as idempotent.

```python
query = Query("SELECT * FROM users")
print(query.is_idempotent())  # False by default
```

**Returns:** `True` if query is idempotent

---

#### `set_idempotent(idempotent: bool) -> None`

Mark the query as idempotent (safe to retry).

```python
query = Query("SELECT * FROM users")
query.set_idempotent(True)
```

**Parameters:**

- `idempotent` - `True` if query is safe to retry

---

#### `get_contents() -> str`

Get the query string.

```python
query = Query("SELECT * FROM users")
print(query.get_contents())  # "SELECT * FROM users"
```

**Returns:** The CQL query string

---

### Complete Example

```python
from rsylla import Query

# Create query with all options
query = (
    Query("SELECT * FROM users WHERE status = ?")
    .with_consistency("LOCAL_QUORUM")
    .with_page_size(500)
    .with_timeout(10000)
)
query.set_idempotent(True)

# Execute
result = await session.query(query, {"status": "active"})
```

---

## PreparedStatement

`PreparedStatement` represents a pre-compiled CQL statement for optimal performance.

### Creating Prepared Statements

Prepared statements are created using `session.prepare()`:

```python
prepared = await session.prepare(
    "INSERT INTO users (id, name, email) VALUES (?, ?, ?)"
)
```

### Methods

#### `with_consistency(consistency: str) -> PreparedStatement`

Set the consistency level.

```python
prepared = await session.prepare("SELECT * FROM users WHERE id = ?")
prepared = prepared.with_consistency("QUORUM")
```

**Parameters:**

- `consistency` - Consistency level string

**Returns:** New `PreparedStatement` with updated settings

---

#### `with_serial_consistency(serial_consistency: str) -> PreparedStatement`

Set the serial consistency level.

```python
prepared = prepared.with_serial_consistency("LOCAL_SERIAL")
```

**Parameters:**

- `serial_consistency` - Serial consistency level string

**Returns:** New `PreparedStatement` with updated settings

---

#### `with_page_size(page_size: int) -> PreparedStatement`

Set the page size.

```python
prepared = prepared.with_page_size(1000)
```

**Parameters:**

- `page_size` - Number of rows per page

**Returns:** New `PreparedStatement` with updated settings

---

#### `with_timestamp(timestamp: int) -> PreparedStatement`

Set a specific timestamp.

```python
prepared = prepared.with_timestamp(1234567890000)
```

**Parameters:**

- `timestamp` - Timestamp in microseconds

**Returns:** New `PreparedStatement` with updated settings

---

#### `with_tracing(tracing: bool) -> PreparedStatement`

Enable or disable tracing.

```python
prepared = prepared.with_tracing(True)
```

**Parameters:**

- `tracing` - `True` to enable tracing

**Returns:** New `PreparedStatement` with updated settings

---

#### `is_idempotent() -> bool`

Check if the statement is idempotent.

```python
if prepared.is_idempotent():
    print("Safe to retry")
```

**Returns:** `True` if statement is idempotent

---

#### `set_idempotent(idempotent: bool) -> PreparedStatement`

Mark the statement as idempotent.

```python
prepared = prepared.set_idempotent(True)
```

**Parameters:**

- `idempotent` - `True` if safe to retry

**Returns:** New `PreparedStatement` with updated settings

---

#### `get_id() -> bytes`

Get the prepared statement ID.

```python
stmt_id = prepared.get_id()
print(f"Statement ID: {stmt_id.hex()}")
```

**Returns:** Statement ID as bytes

---

#### `get_statement() -> str`

Get the original query string.

```python
print(prepared.get_statement())
# "INSERT INTO users (id, name, email) VALUES (?, ?, ?)"
```

**Returns:** The CQL query string

---

### Usage Patterns

#### Prepare Once, Execute Many

```python
# Prepare statement once
insert_stmt = await session.prepare(
    "INSERT INTO users (id, name, email) VALUES (?, ?, ?)"
)

# Execute many times
for user in users:
    await session.execute_prepared(insert_stmt, {
        "id": user["id"],
        "name": user["name"],
        "email": user["email"]
    })
```

#### Statement Repository

```python
class UserRepository:
    """Repository with prepared statements"""

    def __init__(self, session):
        self.session = session
        self._prepared = False

    async def _ensure_prepared(self):
        if self._prepared:
            return

        self.insert_stmt = await self.session.prepare(
            "INSERT INTO users (id, name, email) VALUES (?, ?, ?)"
        )

        self.get_stmt = await self.session.prepare(
            "SELECT * FROM users WHERE id = ?"
        )
        self.get_stmt = self.get_stmt.set_idempotent(True)

        self.update_stmt = await self.session.prepare(
            "UPDATE users SET email = ? WHERE id = ?"
        )

        self._prepared = True

    async def create_user(self, user_id, name, email):
        await self._ensure_prepared()
        await self.session.execute_prepared(self.insert_stmt, {
            "id": user_id,
            "name": name,
            "email": email
        })

    async def get_user(self, user_id):
        await self._ensure_prepared()
        result = await self.session.execute_prepared(
            self.get_stmt,
            {"id": user_id}
        )
        return result.first_row()
```

---

## Consistency Levels

### Standard Consistency

| Level | Description | Use Case |
|-------|-------------|----------|
| `ANY` | Write succeeds if any node acknowledges | Fire-and-forget writes |
| `ONE` | One replica responds | Low latency reads/writes |
| `TWO` | Two replicas respond | Higher durability |
| `THREE` | Three replicas respond | Maximum durability |
| `QUORUM` | Majority of replicas | Strong consistency |
| `ALL` | All replicas respond | Strictest consistency |
| `LOCAL_QUORUM` | Majority in local DC | Multi-DC deployments |
| `EACH_QUORUM` | Majority in each DC | Multi-DC strong consistency |
| `LOCAL_ONE` | One replica in local DC | Low latency in multi-DC |

### Serial Consistency (LWT)

| Level | Description |
|-------|-------------|
| `SERIAL` | Paxos consensus across all DCs |
| `LOCAL_SERIAL` | Paxos consensus in local DC |

### Choosing Consistency

```python
# High availability reads
read_query = Query("SELECT ...").with_consistency("LOCAL_ONE")

# Durable writes
write_query = Query("INSERT ...").with_consistency("LOCAL_QUORUM")

# Strong consistency (read-your-writes)
# Write with QUORUM, read with QUORUM

# Lightweight transactions
lwt_query = (
    Query("UPDATE ... IF EXISTS")
    .with_consistency("QUORUM")
    .with_serial_consistency("SERIAL")
)
```

---

## Tracing

Use tracing to debug slow queries:

```python
query = Query("SELECT * FROM users").with_tracing(True)
result = await session.query(query)

trace_id = result.tracing_id()
if trace_id:
    print(f"Trace ID: {trace_id}")

    # Query trace details
    trace_result = await session.execute(
        """SELECT * FROM system_traces.sessions
           WHERE session_id = ?""",
        {"session_id": trace_id}
    )
    print(trace_result.first_row())
```
