# Batch API

The `Batch` class allows you to execute multiple statements atomically or as a group for better performance.

## Batch

### Constructor

```python
from rsylla import Batch

batch = Batch("logged")  # or "unlogged" or "counter"
```

**Parameters:**

- `batch_type` - Type of batch: `"logged"`, `"unlogged"`, or `"counter"`

**Raises:** `ValueError` for invalid batch type

### Batch Types

| Type | Description | Use Case |
|------|-------------|----------|
| `logged` | Atomic, all-or-nothing | Financial transactions, critical updates |
| `unlogged` | No atomicity guarantee | Log entries, metrics |
| `counter` | Counter updates only | View counts, statistics |

### Methods

#### `append_statement(query: str) -> None`

Add a CQL statement to the batch.

```python
batch = Batch("logged")
batch.append_statement("INSERT INTO users (id, name) VALUES (?, ?)")
batch.append_statement("INSERT INTO users (id, name) VALUES (?, ?)")
```

#### `append_query(query: Query) -> None`

Add a Query object to the batch.

```python
from rsylla import Query

query = Query("INSERT INTO users (id, name) VALUES (?, ?)")
batch.append_query(query)
```

#### `append_prepared(prepared: PreparedStatement) -> None`

Add a prepared statement to the batch.

```python
prepared = await session.prepare("INSERT INTO users (id, name) VALUES (?, ?)")
batch.append_prepared(prepared)
```

#### `with_consistency(consistency: str) -> Batch`

Set the consistency level for the batch.

```python
batch = Batch("logged").with_consistency("QUORUM")
```

#### `with_serial_consistency(serial_consistency: str) -> Batch`

Set the serial consistency level (for LWT in batch).

```python
batch = batch.with_serial_consistency("SERIAL")
```

#### `with_timestamp(timestamp: int) -> Batch`

Set a timestamp for all statements in the batch.

```python
batch = batch.with_timestamp(1234567890000)
```

#### `with_tracing(tracing: bool) -> Batch`

Enable or disable tracing.

```python
batch = batch.with_tracing(True)
```

#### `is_idempotent() -> bool`

Check if the batch is idempotent.

#### `set_idempotent(idempotent: bool) -> None`

Mark the batch as idempotent.

#### `statements_count() -> int`

Get the number of statements in the batch.

```python
print(f"Batch has {batch.statements_count()} statements")
```

## Executing Batches

Use `session.batch()` to execute:

```python
result = await session.batch(batch, values)
```

### Example

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

## Best Practices

- **Batch same partition** - Group statements that affect the same partition key
- **Keep batches small** - Large batches can timeout or overwhelm coordinators
- **Use unlogged when possible** - If atomicity isn't required, unlogged is faster
- **Don't batch across many partitions** - This defeats the purpose and is slower
