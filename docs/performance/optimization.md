# Optimization Tips

Maximize rsylla performance with these optimization strategies.

## Connection Optimization

### Pool Size

```python
# Tune based on workload
session = await (
    SessionBuilder()
    .pool_size(20)  # Per host
    .build()
)
```

**Guidelines:**

- Start with 10-20 connections per host
- Increase for high-throughput workloads
- Monitor connection usage

### Compression

```python
session = await (
    SessionBuilder()
    .compression("lz4")  # Faster
    # .compression("snappy")  # Better ratio
    .build()
)
```

**When to use:**

- Large payloads (>1KB)
- Limited network bandwidth
- Not for tiny operations (adds overhead)

### TCP Nodelay

```python
session = await (
    SessionBuilder()
    .tcp_nodelay(True)
    .build()
)
```

Disables Nagle's algorithm for lower latency.

## Query Optimization

### Use Prepared Statements

```python
# Prepare once
stmt = await session.prepare("SELECT * FROM users WHERE id = ?")

# Execute many times
for user_id in user_ids:
    result = await session.execute_prepared(stmt, {"id": user_id})
```

**Benefits:**

- Query parsed once
- Reduced network overhead
- ~15-20% faster than unprepared

### Appropriate Consistency

```python
# Fast reads (eventual consistency)
query = Query("SELECT ...").with_consistency("LOCAL_ONE")

# Durable writes
query = Query("INSERT ...").with_consistency("LOCAL_QUORUM")
```

**Guidelines:**

- Use `LOCAL_ONE` for non-critical reads
- Use `LOCAL_QUORUM` for important writes
- Avoid `ALL` unless necessary

### Mark Idempotent Queries

```python
stmt = await session.prepare("SELECT * FROM users WHERE id = ?")
stmt = stmt.set_idempotent(True)
```

Enables safe retries for transient failures.

### Use Paging

```python
query = Query("SELECT * FROM large_table").with_page_size(1000)
result = await session.query(query)

for row in result:  # Automatic paging
    process(row)
```

**Guidelines:**

- Set page size based on row size
- Smaller pages = more round trips
- Larger pages = more memory

## Data Model Optimization

### Partition Design

```python
# GOOD: Distribute data evenly
PRIMARY KEY ((user_id, date), event_time)

# BAD: Hot partitions
PRIMARY KEY (date, event_time)  # All data for one date in one partition
```

### Clustering Order

```python
# For queries that need latest first
WITH CLUSTERING ORDER BY (created_at DESC)
```

### Use TTL

```python
await session.execute(
    "INSERT INTO sessions (id, data) VALUES (?, ?) USING TTL ?",
    {"id": session_id, "data": data, "ttl": 3600}
)
```

Automatic cleanup of temporary data.

## Batch Optimization

### Batch Same Partition

```python
# GOOD: Same partition
batch = Batch("logged")
batch.append_statement("INSERT INTO user_data (user_id, ...) VALUES (1, ...)")
batch.append_statement("INSERT INTO user_data (user_id, ...) VALUES (1, ...)")

# BAD: Different partitions
for user_id in range(1000):
    batch.append_statement("INSERT INTO users ...")
```

### Use Unlogged Batches

```python
# When atomicity not required
batch = Batch("unlogged")  # Faster
```

### Keep Batches Small

- Max 100 statements per batch
- Large batches can timeout
- Split into smaller batches if needed

## Async Optimization

### Concurrent Requests

```python
import asyncio

async def fetch_users(session, user_ids):
    tasks = [
        session.execute("SELECT * FROM users WHERE id = ?", {"id": uid})
        for uid in user_ids
    ]
    return await asyncio.gather(*tasks)
```

### Connection Reuse

```python
# GOOD: Share session
class Database:
    _session = None

    @classmethod
    async def get_session(cls):
        if cls._session is None:
            cls._session = await Session.connect(["..."])
        return cls._session
```

## Monitoring

### Enable Tracing (Debug Only)

```python
query = Query("SELECT ...").with_tracing(True)
result = await session.query(query)

if result.tracing_id():
    print(f"Trace: {result.tracing_id()}")
```

### Check Warnings

```python
result = await session.execute("...")
for warning in result.warnings():
    print(f"Warning: {warning}")
```

## Anti-Patterns to Avoid

### Don't Create Sessions Per Request

```python
# BAD
async def get_user(user_id):
    session = await Session.connect([...])  # Slow!
    return await session.execute(...)
```

### Don't Use ALLOW FILTERING

```python
# BAD
await session.execute("SELECT * FROM users WHERE email = ? ALLOW FILTERING")
```

### Don't Use SELECT COUNT(*)

```python
# BAD
await session.execute("SELECT COUNT(*) FROM large_table")
```

### Don't Use Large IN Clauses

```python
# BAD
await session.execute("SELECT * FROM users WHERE id IN (1,2,3,...,1000)")
```

## Performance Checklist

- [ ] Use connection pooling
- [ ] Enable compression for large data
- [ ] Use prepared statements
- [ ] Set appropriate consistency levels
- [ ] Mark idempotent queries
- [ ] Use paging for large results
- [ ] Batch same-partition operations
- [ ] Monitor latencies and errors
