# Errors

rsylla uses `ScyllaError` as the primary exception for all database-related errors.

## ScyllaError

All rsylla operations can raise `ScyllaError`:

```python
from rsylla import ScyllaError

try:
    result = await session.execute("INVALID QUERY")
except ScyllaError as e:
    print(f"Database error: {e}")
```

### Common Error Scenarios

#### Connection Errors

```python
try:
    session = await Session.connect(["invalid-host:9042"])
except ScyllaError as e:
    print(f"Connection failed: {e}")
```

#### Query Errors

```python
try:
    # Invalid CQL syntax
    await session.execute("SELEC * FROM users")
except ScyllaError as e:
    print(f"Query error: {e}")
```

#### Keyspace Errors

```python
try:
    await session.use_keyspace("nonexistent", False)
except ScyllaError as e:
    print(f"Keyspace error: {e}")
```

#### Timeout Errors

```python
try:
    query = Query("SELECT * FROM large_table").with_timeout(100)
    await session.query(query)
except ScyllaError as e:
    print(f"Timeout: {e}")
```

## Error Handling Patterns

### Basic Try/Except

```python
from rsylla import Session, ScyllaError

async def get_user(session, user_id):
    try:
        result = await session.execute(
            "SELECT * FROM users WHERE id = ?",
            {"id": user_id}
        )
        return result.first_row()
    except ScyllaError as e:
        print(f"Failed to get user: {e}")
        return None
```

### Retry with Backoff

```python
import asyncio
from rsylla import ScyllaError

async def execute_with_retry(session, query, values=None, max_retries=3):
    """Execute query with exponential backoff"""
    for attempt in range(max_retries):
        try:
            return await session.execute(query, values)
        except ScyllaError as e:
            if attempt == max_retries - 1:
                raise  # Re-raise on last attempt

            wait_time = 2 ** attempt
            print(f"Retry {attempt + 1}/{max_retries} in {wait_time}s: {e}")
            await asyncio.sleep(wait_time)
```

### Graceful Degradation

```python
async def get_user_or_default(session, user_id):
    """Return user or default value on error"""
    try:
        result = await session.execute(
            "SELECT * FROM users WHERE id = ?",
            {"id": user_id}
        )
        row = result.first_row()
        if row:
            return {"id": row[0], "name": row[1], "email": row[2]}
        return None
    except ScyllaError:
        # Return cached/default value
        return {"id": user_id, "name": "Unknown", "email": ""}
```

### Logging Errors

```python
import logging
from rsylla import ScyllaError

logger = logging.getLogger(__name__)

async def execute_logged(session, query, values=None):
    """Execute with error logging"""
    try:
        return await session.execute(query, values)
    except ScyllaError as e:
        logger.error(f"Query failed: {query[:100]}... Error: {e}")
        raise
```

## Python Standard Exceptions

rsylla may also raise standard Python exceptions:

| Exception | When |
|-----------|------|
| `ValueError` | Invalid parameter values (e.g., invalid consistency level, batch type) |
| `IndexError` | Row column access out of range |
| `TypeError` | Wrong parameter types |

### ValueError Example

```python
from rsylla import Query

try:
    query = Query("SELECT * FROM users").with_consistency("INVALID")
except ValueError as e:
    print(f"Invalid consistency: {e}")
```

### IndexError Example

```python
row = result.first_row()
try:
    value = row[100]  # Out of range
except IndexError as e:
    print(f"Column not found: {e}")
```

## Best Practices

1. **Always handle ScyllaError** for database operations
2. **Use specific error handling** when possible
3. **Implement retry logic** for transient failures
4. **Log errors** for debugging
5. **Provide fallbacks** for non-critical operations
6. **Don't catch too broadly** - let unexpected errors propagate
