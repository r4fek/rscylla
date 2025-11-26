# Advanced Patterns

Advanced usage patterns for complex applications.

## Multi-Tenant Architecture

```python
class MultiTenantDB:
    """Manage per-tenant keyspaces"""

    def __init__(self, session):
        self.session = session
        self._tenant_stmts = {}

    async def create_tenant(self, tenant_id):
        """Create tenant keyspace and tables"""
        await self.session.execute(f"""
            CREATE KEYSPACE IF NOT EXISTS tenant_{tenant_id}
            WITH replication = {{'class': 'NetworkTopologyStrategy', 'dc1': 3}}
        """)

        await self.session.execute(f"""
            CREATE TABLE IF NOT EXISTS tenant_{tenant_id}.users (
                id int PRIMARY KEY,
                name text,
                email text
            )
        """)

    async def get_user(self, tenant_id, user_id):
        """Get user for specific tenant"""
        key = f"get_user_{tenant_id}"

        if key not in self._tenant_stmts:
            self._tenant_stmts[key] = await self.session.prepare(
                f"SELECT * FROM tenant_{tenant_id}.users WHERE id = ?"
            )

        result = await self.session.execute_prepared(
            self._tenant_stmts[key], {"id": user_id}
        )
        return result.first_row()
```

## Time Series with Bucketing

```python
class TimeSeriesStore:
    """Efficient time series storage with bucketing"""

    def __init__(self, session):
        self.session = session

    async def write(self, metric, value, timestamp=None):
        """Write metric with automatic bucketing"""
        import time
        from datetime import date

        ts = timestamp or int(time.time() * 1000)
        bucket = (date.today() - date(1970, 1, 1)).days

        await self.session.execute(
            """INSERT INTO metrics (name, bucket, ts, value)
               VALUES (?, ?, ?, ?) USING TTL 604800""",
            {"name": metric, "bucket": bucket, "ts": ts, "value": value}
        )

    async def query_range(self, metric, start, end):
        """Query metrics across date buckets"""
        from datetime import date

        start_bucket = (start.date() - date(1970, 1, 1)).days
        end_bucket = (end.date() - date(1970, 1, 1)).days

        results = []
        for bucket in range(start_bucket, end_bucket + 1):
            result = await self.session.execute(
                """SELECT ts, value FROM metrics
                   WHERE name = ? AND bucket = ?
                   AND ts >= ? AND ts <= ?""",
                {
                    "name": metric,
                    "bucket": bucket,
                    "ts_start": int(start.timestamp() * 1000),
                    "ts_end": int(end.timestamp() * 1000)
                }
            )
            results.extend(result.rows())

        return results
```

## Event Sourcing

```python
import json

class EventStore:
    """Event sourcing with optimistic locking"""

    def __init__(self, session):
        self.session = session

    async def append(self, stream_id, event_type, data, expected_version=None):
        """Append event with version check"""
        import time

        # Check current version
        result = await self.session.execute(
            "SELECT version FROM events WHERE stream_id = ? ORDER BY version DESC LIMIT 1",
            {"stream_id": stream_id}
        )
        row = result.first_row()
        current = row[0] if row else 0

        if expected_version is not None and current != expected_version:
            raise Exception(f"Version conflict: expected {expected_version}, got {current}")

        new_version = current + 1

        # Use LWT for safe insert
        result = await self.session.execute(
            """INSERT INTO events (stream_id, version, type, data, ts)
               VALUES (?, ?, ?, ?, ?) IF NOT EXISTS""",
            {
                "stream_id": stream_id,
                "version": new_version,
                "type": event_type,
                "data": json.dumps(data),
                "ts": int(time.time() * 1000)
            }
        )

        row = result.first_row()
        if row and not row[0]:  # [applied] = false
            raise Exception("Concurrent modification")

        return new_version

    async def read_stream(self, stream_id, from_version=0):
        """Read all events from stream"""
        result = await self.session.execute(
            """SELECT version, type, data FROM events
               WHERE stream_id = ? AND version > ?""",
            {"stream_id": stream_id, "version": from_version}
        )

        return [
            {
                "version": row[0],
                "type": row[1],
                "data": json.loads(row[2])
            }
            for row in result
        ]
```

## Materialized Views Pattern

```python
class MaterializedViewManager:
    """Maintain denormalized views"""

    def __init__(self, session):
        self.session = session

    async def create_post(self, post_id, author_id, title, tags):
        """Create post and update all views atomically"""
        import time
        ts = int(time.time() * 1000)

        batch = Batch("logged")

        # Main table
        batch.append_statement(
            "INSERT INTO posts (id, author, title, tags, created) VALUES (?, ?, ?, ?, ?)"
        )

        # By author view
        batch.append_statement(
            "INSERT INTO posts_by_author (author, created, id, title) VALUES (?, ?, ?, ?)"
        )

        # By tag views
        for _ in tags:
            batch.append_statement(
                "INSERT INTO posts_by_tag (tag, created, id, title) VALUES (?, ?, ?, ?)"
            )

        values = [
            {"id": post_id, "author": author_id, "title": title, "tags": tags, "created": ts},
            {"author": author_id, "created": ts, "id": post_id, "title": title}
        ]

        for tag in tags:
            values.append({"tag": tag, "created": ts, "id": post_id, "title": title})

        await self.session.batch(batch, values)
```

## Distributed Locking

```python
import uuid
import time

class DistributedLock:
    """Distributed lock using LWT"""

    def __init__(self, session, ttl_seconds=30):
        self.session = session
        self.ttl = ttl_seconds

    async def acquire(self, lock_name, holder=None):
        """Try to acquire lock"""
        holder = holder or str(uuid.uuid4())
        now = int(time.time() * 1000)

        result = await self.session.execute(
            """INSERT INTO locks (name, holder, acquired_at)
               VALUES (?, ?, ?)
               IF NOT EXISTS
               USING TTL ?""",
            {
                "name": lock_name,
                "holder": holder,
                "acquired_at": now,
                "ttl": self.ttl
            }
        )

        row = result.first_row()
        if row and row[0]:  # [applied]
            return holder
        return None

    async def release(self, lock_name, holder):
        """Release lock if we own it"""
        await self.session.execute(
            "DELETE FROM locks WHERE name = ? IF holder = ?",
            {"name": lock_name, "holder": holder}
        )

    async def with_lock(self, lock_name, callback):
        """Execute callback with lock held"""
        holder = await self.acquire(lock_name)
        if not holder:
            raise Exception(f"Could not acquire lock: {lock_name}")

        try:
            return await callback()
        finally:
            await self.release(lock_name, holder)
```

## Read-Through Cache

```python
import pickle

class CachedRepository:
    """Repository with caching layer"""

    def __init__(self, session, cache_ttl=300):
        self.session = session
        self.cache_ttl = cache_ttl

    async def get_user(self, user_id):
        """Get user with caching"""
        cache_key = f"user:{user_id}"

        # Try cache first
        result = await self.session.execute(
            "SELECT value FROM cache WHERE key = ?",
            {"key": cache_key}
        )
        row = result.first_row()
        if row:
            return pickle.loads(row[0])

        # Cache miss - fetch from source
        result = await self.session.execute(
            "SELECT * FROM users WHERE id = ?",
            {"id": user_id}
        )
        row = result.first_row()
        if not row:
            return None

        user = {"id": row[0], "name": row[1], "email": row[2]}

        # Store in cache
        await self.session.execute(
            "INSERT INTO cache (key, value) VALUES (?, ?) USING TTL ?",
            {"key": cache_key, "value": pickle.dumps(user), "ttl": self.cache_ttl}
        )

        return user

    async def invalidate(self, user_id):
        """Invalidate cache entry"""
        await self.session.execute(
            "DELETE FROM cache WHERE key = ?",
            {"key": f"user:{user_id}"}
        )
```

## Saga Pattern

```python
class OrderSaga:
    """Distributed transaction using saga pattern"""

    def __init__(self, session):
        self.session = session

    async def create_order(self, order_id, user_id, items):
        """Create order with compensating transactions"""
        try:
            # Step 1: Reserve inventory
            for item in items:
                await self._reserve_inventory(item["id"], item["qty"])

            # Step 2: Create order
            await self._create_order(order_id, user_id, items)

            # Step 3: Process payment
            await self._process_payment(user_id, sum(i["price"] for i in items))

            # Step 4: Confirm order
            await self._confirm_order(order_id)

        except Exception as e:
            # Compensate: release inventory
            for item in items:
                await self._release_inventory(item["id"], item["qty"])

            # Compensate: cancel order
            await self._cancel_order(order_id)

            raise

    async def _reserve_inventory(self, item_id, qty):
        result = await self.session.execute(
            "UPDATE inventory SET reserved = reserved + ? WHERE id = ? IF quantity >= ?",
            {"qty": qty, "id": item_id, "min_qty": qty}
        )
        if not result.first_row()[0]:
            raise Exception(f"Insufficient inventory for {item_id}")

    async def _release_inventory(self, item_id, qty):
        await self.session.execute(
            "UPDATE inventory SET reserved = reserved - ? WHERE id = ?",
            {"qty": qty, "id": item_id}
        )
```
