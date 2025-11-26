# Best Practices

Production-ready patterns and recommendations for using rsylla effectively.

## Connection Management

### Reuse Sessions

```python
# GOOD: Create session once
session = await Session.connect(["node:9042"])

async def get_user(user_id):
    return await session.execute("SELECT ...", {"id": user_id})

# BAD: Create new session per query
async def get_user_bad(user_id):
    session = await Session.connect(["node:9042"])  # Slow!
    return await session.execute("SELECT ...", {"id": user_id})
```

### Configure for Production

```python
session = await (
    SessionBuilder()
    .known_nodes(["node1:9042", "node2:9042", "node3:9042"])
    .pool_size(20)
    .connection_timeout(10000)
    .compression("lz4")
    .tcp_nodelay(True)
    .user("app", "password")
    .build()
)
```

## Query Optimization

### Use Prepared Statements

```python
# GOOD: Prepare once, execute many
stmt = await session.prepare("INSERT INTO users ...")
for user in users:
    await session.execute_prepared(stmt, user)

# BAD: Parse query every time
for user in users:
    await session.execute("INSERT INTO users ...", user)
```

### Select Only Needed Columns

```python
# GOOD
result = await session.execute("SELECT id, name FROM users WHERE id = ?")

# BAD
result = await session.execute("SELECT * FROM users WHERE id = ?")
```

### Set Appropriate Consistency

```python
# Fast reads
read_query = Query("SELECT ...").with_consistency("LOCAL_ONE")

# Durable writes
write_query = Query("INSERT ...").with_consistency("LOCAL_QUORUM")
```

## Data Modeling

### Design for Queries

```python
# GOOD: Table designed for query pattern
CREATE TABLE posts_by_user (
    user_id int,
    created_at timestamp,
    post_id uuid,
    title text,
    PRIMARY KEY (user_id, created_at)
) WITH CLUSTERING ORDER BY (created_at DESC)

# Query: "Get latest posts for user"
SELECT * FROM posts_by_user WHERE user_id = ? LIMIT 10

# BAD: Requires ALLOW FILTERING
SELECT * FROM posts WHERE user_id = ? ALLOW FILTERING
```

### Avoid Large Partitions

```python
# GOOD: Partition by user + date
PRIMARY KEY ((user_id, event_date), event_time)

# BAD: Unbounded partition
PRIMARY KEY (user_id, event_time)  # Grows forever!
```

## Batch Operations

### Batch Same Partition

```python
# GOOD: Same partition key
batch = Batch("logged")
batch.append_statement("INSERT INTO user_data (user_id, ...) VALUES (?, ...)")
batch.append_statement("UPDATE user_stats SET ... WHERE user_id = ?")
await session.batch(batch, [...])

# BAD: Different partitions
for user_id in range(1000):  # 1000 partitions!
    batch.append_statement("INSERT INTO users ...")
```

### Use Unlogged When Possible

```python
# If atomicity not required, unlogged is faster
batch = Batch("unlogged")
```

## Error Handling

### Implement Retry Logic

```python
async def execute_with_retry(session, query, values, retries=3):
    for attempt in range(retries):
        try:
            return await session.execute(query, values)
        except ScyllaError as e:
            if attempt == retries - 1:
                raise
            await asyncio.sleep(2 ** attempt)
```

### Handle Specific Errors

```python
try:
    await session.execute("...")
except ScyllaError as e:
    logger.error(f"Query failed: {e}")
    # Handle gracefully
```

## Performance

### Enable Compression

```python
session = await (
    SessionBuilder()
    .compression("lz4")  # Or "snappy"
    .build()
)
```

### Use Paging for Large Results

```python
query = Query("SELECT * FROM large_table").with_page_size(1000)
result = await session.query(query)

for row in result:  # Handles paging automatically
    process(row)
```

### Mark Idempotent Queries

```python
# Enables safe retries
query = Query("SELECT ...").set_idempotent(True)
```

## Anti-Patterns to Avoid

### Never Use ALLOW FILTERING

```python
# BAD
await session.execute("SELECT * FROM users WHERE email = ? ALLOW FILTERING")

# GOOD: Create proper table
CREATE TABLE users_by_email (email text PRIMARY KEY, user_id int)
```

### Avoid SELECT COUNT(*)

```python
# BAD: Scans entire table
result = await session.execute("SELECT COUNT(*) FROM users")

# GOOD: Use counters
UPDATE user_count SET count = count + 1 WHERE type = 'total'
```

### Don't Use IN with Many Values

```python
# BAD
SELECT * FROM users WHERE id IN (1, 2, 3, ..., 1000)

# GOOD: Individual queries or batches
for user_id in ids:
    result = await session.execute("SELECT * FROM users WHERE id = ?", {"id": user_id})
```

## Production Checklist

### Configuration
- [ ] Use multiple known nodes
- [ ] Set appropriate pool size
- [ ] Enable compression
- [ ] Configure timeouts
- [ ] Enable TCP nodelay

### Queries
- [ ] Use prepared statements
- [ ] Set appropriate consistency levels
- [ ] Mark idempotent queries
- [ ] Use paging for large results

### Error Handling
- [ ] Implement retry logic
- [ ] Log errors appropriately
- [ ] Handle connection failures

### Monitoring
- [ ] Track query latencies
- [ ] Monitor error rates
- [ ] Alert on failures
