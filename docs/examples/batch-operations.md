# Batch Operations Examples

Batches allow you to execute multiple statements atomically or efficiently.

## Basic Batch

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

## Batch Types

### Logged Batch (Atomic)

```python
# All statements succeed or all fail
batch = Batch("logged")
batch.append_statement("UPDATE accounts SET balance = balance - ? WHERE id = ?")
batch.append_statement("UPDATE accounts SET balance = balance + ? WHERE id = ?")

await session.batch(batch, [
    {"balance": 100, "id": "from_account"},
    {"balance": 100, "id": "to_account"}
])
```

### Unlogged Batch (Performance)

```python
# No atomicity guarantee, but faster
batch = Batch("unlogged")
for _ in range(100):
    batch.append_statement("INSERT INTO logs (id, msg, ts) VALUES (?, ?, ?)")

await session.batch(batch, log_entries)
```

### Counter Batch

```python
batch = Batch("counter")
batch.append_statement("UPDATE stats SET views = views + ? WHERE page = ?")
batch.append_statement("UPDATE stats SET clicks = clicks + ? WHERE page = ?")

await session.batch(batch, [
    {"views": 1, "page": "home"},
    {"clicks": 1, "page": "home"}
])
```

## With Prepared Statements

```python
insert_stmt = await session.prepare(
    "INSERT INTO events (user_id, ts, type) VALUES (?, ?, ?)"
)

batch = Batch("unlogged")
batch.append_prepared(insert_stmt)
batch.append_prepared(insert_stmt)
batch.append_prepared(insert_stmt)

await session.batch(batch, [
    {"user_id": 1, "ts": now, "type": "login"},
    {"user_id": 1, "ts": now, "type": "view"},
    {"user_id": 1, "ts": now, "type": "click"}
])
```

## Batch Configuration

```python
batch = (
    Batch("logged")
    .with_consistency("QUORUM")
    .with_timestamp(custom_timestamp)
    .with_tracing(True)
)
batch.set_idempotent(True)
```

## Multi-Table Atomic Update

```python
async def create_order(session, order_id, user_id, items):
    """Create order and update inventory atomically"""

    batch = Batch("logged")

    # Insert order
    batch.append_statement(
        "INSERT INTO orders (id, user_id, status, created) VALUES (?, ?, ?, ?)"
    )

    # Insert order items
    for _ in items:
        batch.append_statement(
            "INSERT INTO order_items (order_id, item_id, qty) VALUES (?, ?, ?)"
        )

    # Update inventory
    for _ in items:
        batch.append_statement(
            "UPDATE inventory SET quantity = quantity - ? WHERE item_id = ?"
        )

    values = [
        {"id": order_id, "user_id": user_id, "status": "pending", "created": now}
    ]
    for item in items:
        values.append({"order_id": order_id, "item_id": item["id"], "qty": item["qty"]})
    for item in items:
        values.append({"quantity": item["qty"], "item_id": item["id"]})

    await session.batch(batch, values)
```

## Bulk Insert Pattern

```python
async def bulk_insert(session, table, rows, batch_size=100):
    """Insert rows in batches"""
    stmt = await session.prepare(f"INSERT INTO {table} JSON ?")

    for i in range(0, len(rows), batch_size):
        chunk = rows[i:i + batch_size]

        batch = Batch("unlogged")
        for _ in chunk:
            batch.append_prepared(stmt)

        await session.batch(batch, [{"json": json.dumps(r)} for r in chunk])

    print(f"Inserted {len(rows)} rows")
```

## Best Practices

1. **Batch same partition** - Most efficient when all statements target same partition
2. **Keep batches small** - Large batches (>100 statements) can timeout
3. **Use unlogged for performance** - When atomicity isn't required
4. **Don't batch across many partitions** - Defeats the purpose
5. **Use prepared statements** - For better performance in batches
