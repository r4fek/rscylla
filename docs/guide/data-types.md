# Data Types Guide

This guide explains how CQL data types map to Python types in rsylla.

## Basic Types

### Integer Types

| CQL Type | Python Type | Range |
|----------|-------------|-------|
| `tinyint` | `int` | -128 to 127 |
| `smallint` | `int` | -32,768 to 32,767 |
| `int` | `int` | -2^31 to 2^31-1 |
| `bigint` | `int` | -2^63 to 2^63-1 |

```python
await session.execute(
    "INSERT INTO numbers (id, tiny, big) VALUES (?, ?, ?)",
    {"id": 1, "tiny": 42, "big": 9223372036854775807}
)
```

### Floating Point

| CQL Type | Python Type |
|----------|-------------|
| `float` | `float` |
| `double` | `float` |

```python
await session.execute(
    "INSERT INTO measurements (id, temp) VALUES (?, ?)",
    {"id": 1, "temp": 22.5}
)
```

### Text Types

| CQL Type | Python Type |
|----------|-------------|
| `text` | `str` |
| `varchar` | `str` |
| `ascii` | `str` |

```python
await session.execute(
    "INSERT INTO messages (id, content) VALUES (?, ?)",
    {"id": 1, "content": "Hello, World!"}
)
```

### Boolean

| CQL Type | Python Type |
|----------|-------------|
| `boolean` | `bool` |

```python
await session.execute(
    "INSERT INTO users (id, active) VALUES (?, ?)",
    {"id": 1, "active": True}
)
```

### Binary Data

| CQL Type | Python Type |
|----------|-------------|
| `blob` | `bytes` |

```python
await session.execute(
    "INSERT INTO files (id, data) VALUES (?, ?)",
    {"id": 1, "data": b'\x00\x01\x02\x03'}
)
```

## Temporal Types

### Timestamp

| CQL Type | Python Type |
|----------|-------------|
| `timestamp` | `int` (milliseconds since epoch) |

```python
import time

now = int(time.time() * 1000)
await session.execute(
    "INSERT INTO events (id, ts) VALUES (?, ?)",
    {"id": 1, "ts": now}
)

# Reading
result = await session.execute("SELECT ts FROM events WHERE id = ?", {"id": 1})
ts_ms = result.first_row()[0]
from datetime import datetime
dt = datetime.fromtimestamp(ts_ms / 1000)
```

### Date and Time

| CQL Type | Python Type |
|----------|-------------|
| `date` | `int` (days since epoch) |
| `time` | `int` (nanoseconds since midnight) |

```python
from datetime import date

today = date.today()
days = (today - date(1970, 1, 1)).days

await session.execute(
    "INSERT INTO schedules (id, event_date) VALUES (?, ?)",
    {"id": 1, "event_date": days}
)
```

### Duration

| CQL Type | Python Type |
|----------|-------------|
| `duration` | `dict` with months, days, nanoseconds |

```python
# Reading duration
result = await session.execute("SELECT duration_col FROM table")
duration = result.first_row()[0]
# {"months": 0, "days": 1, "nanoseconds": 3600000000000}
```

## UUID Types

| CQL Type | Python Type |
|----------|-------------|
| `uuid` | `str` |
| `timeuuid` | `str` |

```python
import uuid

await session.execute(
    "INSERT INTO items (id, name) VALUES (?, ?)",
    {"id": str(uuid.uuid4()), "name": "Item 1"}
)
```

## Collection Types

### List

| CQL Type | Python Type |
|----------|-------------|
| `list<T>` | `list` |

```python
await session.execute(
    "INSERT INTO users (id, tags) VALUES (?, ?)",
    {"id": 1, "tags": ["admin", "user", "premium"]}
)

# Append to list
await session.execute(
    "UPDATE users SET tags = tags + ? WHERE id = ?",
    {"tags": ["new_tag"], "id": 1}
)
```

### Set

| CQL Type | Python Type |
|----------|-------------|
| `set<T>` | `list` |

```python
await session.execute(
    "INSERT INTO products (id, categories) VALUES (?, ?)",
    {"id": 1, "categories": ["electronics", "gaming"]}
)
```

### Map

| CQL Type | Python Type |
|----------|-------------|
| `map<K, V>` | `dict` |

```python
await session.execute(
    "INSERT INTO users (id, attributes) VALUES (?, ?)",
    {"id": 1, "attributes": {"city": "NYC", "country": "USA"}}
)

# Reading
result = await session.execute("SELECT attributes FROM users WHERE id = ?", {"id": 1})
attrs = result.first_row()[0]  # dict
print(attrs["city"])  # "NYC"
```

## Advanced Types

### Counter

| CQL Type | Python Type |
|----------|-------------|
| `counter` | `int` |

```python
# Counters can only be incremented/decremented
await session.execute(
    "UPDATE page_views SET views = views + ? WHERE page = ?",
    {"views": 1, "page": "home"}
)
```

### Tuple

| CQL Type | Python Type |
|----------|-------------|
| `tuple<T1, T2, ...>` | `list` |

```python
await session.execute(
    "INSERT INTO locations (id, coords) VALUES (?, ?)",
    {"id": 1, "coords": [40.7128, -74.0060]}  # lat, lon
)
```

### User Defined Types

| CQL Type | Python Type |
|----------|-------------|
| `frozen<UDT>` | `dict` |

```python
await session.execute(
    "INSERT INTO users (id, address) VALUES (?, ?)",
    {
        "id": 1,
        "address": {
            "street": "123 Main St",
            "city": "NYC",
            "zip": 10001
        }
    }
)
```

### Decimal and Varint

| CQL Type | Python Type |
|----------|-------------|
| `decimal` | `str` |
| `varint` | `str` |

These are returned as strings to preserve precision.

## NULL Values

NULL values are represented as `None` in Python:

```python
await session.execute(
    "INSERT INTO users (id, email) VALUES (?, ?)",
    {"id": 1, "email": None}  # NULL
)

result = await session.execute("SELECT email FROM users WHERE id = ?", {"id": 1})
email = result.first_row()[0]
if email is None:
    print("No email set")
```

## Type Conversion Summary

### Python to CQL

| Python | CQL |
|--------|-----|
| `bool` | `boolean` |
| `int` (small) | `int` |
| `int` (large) | `bigint` |
| `float` | `double` |
| `str` | `text` |
| `bytes` | `blob` |
| `list` | `list` or `set` |
| `dict` | `map` |
| `None` | `NULL` |

### CQL to Python

| CQL | Python |
|-----|--------|
| `boolean` | `bool` |
| `tinyint`, `smallint`, `int`, `bigint` | `int` |
| `float`, `double` | `float` |
| `text`, `varchar`, `ascii` | `str` |
| `blob` | `bytes` |
| `uuid`, `timeuuid` | `str` |
| `timestamp` | `int` (ms) |
| `date` | `int` (days) |
| `time` | `int` (ns) |
| `list`, `set` | `list` |
| `map` | `dict` |
| `tuple` | `list` |
| `UDT` | `dict` |
