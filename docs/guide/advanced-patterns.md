# Advanced Patterns

Advanced usage patterns for complex applications.

## Multi-Tenant Architecture

```python
from rsylla import Session, SessionBuilder

class MultiTenantDB:
    """Manage per-tenant keyspaces"""

    def __init__(self, session: Session):
        self.session = session
        self._tenant_stmts: dict = {}

    async def create_tenant(self, tenant_id: str) -> None:
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

    async def get_user(self, tenant_id: str, user_id: int):
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


# Usage
async def main():
    session = await SessionBuilder().known_node("localhost:9042").build()

    db = MultiTenantDB(session)
    await db.create_tenant("acme")
    user = await db.get_user("acme", 123)
```

## Time Series with Bucketing

```python
import time
from datetime import date, datetime
from rsylla import Session

class TimeSeriesStore:
    """Efficient time series storage with bucketing"""

    def __init__(self, session: Session):
        self.session = session
        self._write_stmt = None
        self._read_stmt = None

    async def _ensure_prepared(self) -> None:
        if self._write_stmt is None:
            self._write_stmt = await self.session.prepare(
                """INSERT INTO metrics (name, bucket, ts, value)
                   VALUES (?, ?, ?, ?) USING TTL 604800"""
            )
            self._read_stmt = await self.session.prepare(
                """SELECT ts, value FROM metrics
                   WHERE name = ? AND bucket = ?
                   AND ts >= ? AND ts <= ?"""
            )

    async def write(self, metric: str, value: float, timestamp: int | None = None) -> None:
        """Write metric with automatic bucketing"""
        await self._ensure_prepared()

        ts = timestamp or int(time.time() * 1000)
        bucket = (date.today() - date(1970, 1, 1)).days

        await self.session.execute_prepared(
            self._write_stmt,
            {"name": metric, "bucket": bucket, "ts": ts, "value": value}
        )

    async def query_range(self, metric: str, start: datetime, end: datetime) -> list:
        """Query metrics across date buckets"""
        await self._ensure_prepared()

        start_bucket = (start.date() - date(1970, 1, 1)).days
        end_bucket = (end.date() - date(1970, 1, 1)).days
        ts_start = int(start.timestamp() * 1000)
        ts_end = int(end.timestamp() * 1000)

        results = []
        for bucket in range(start_bucket, end_bucket + 1):
            result = await self.session.execute_prepared(
                self._read_stmt,
                {"name": metric, "bucket": bucket, "ts_start": ts_start, "ts_end": ts_end}
            )
            for row in result:
                results.append({"ts": row[0], "value": row[1]})

        return results
```

## Event Sourcing

```python
import json
import time
from rsylla import Session

class EventStore:
    """Event sourcing with optimistic locking"""

    def __init__(self, session: Session):
        self.session = session
        self._version_stmt = None
        self._append_stmt = None
        self._read_stmt = None

    async def _ensure_prepared(self) -> None:
        if self._version_stmt is None:
            self._version_stmt = await self.session.prepare(
                "SELECT version FROM events WHERE stream_id = ? ORDER BY version DESC LIMIT 1"
            )
            self._append_stmt = await self.session.prepare(
                """INSERT INTO events (stream_id, version, type, data, ts)
                   VALUES (?, ?, ?, ?, ?) IF NOT EXISTS"""
            )
            self._read_stmt = await self.session.prepare(
                """SELECT version, type, data FROM events
                   WHERE stream_id = ? AND version > ?"""
            )

    async def append(
        self,
        stream_id: str,
        event_type: str,
        data: dict,
        expected_version: int | None = None
    ) -> int:
        """Append event with version check"""
        await self._ensure_prepared()

        # Check current version
        result = await self.session.execute_prepared(
            self._version_stmt, {"stream_id": stream_id}
        )
        row = result.first_row()
        current = row[0] if row else 0

        if expected_version is not None and current != expected_version:
            raise Exception(f"Version conflict: expected {expected_version}, got {current}")

        new_version = current + 1
        ts = int(time.time() * 1000)

        # Use LWT for safe insert
        result = await self.session.execute_prepared(
            self._append_stmt,
            {
                "stream_id": stream_id,
                "version": new_version,
                "type": event_type,
                "data": json.dumps(data),
                "ts": ts
            }
        )

        row = result.first_row()
        if row and not row[0]:  # [applied] = false
            raise Exception("Concurrent modification")

        return new_version

    async def read_stream(self, stream_id: str, from_version: int = 0) -> list:
        """Read all events from stream"""
        await self._ensure_prepared()

        result = await self.session.execute_prepared(
            self._read_stmt,
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
import time
from rsylla import Session, Batch

class MaterializedViewManager:
    """Maintain denormalized views"""

    def __init__(self, session: Session):
        self.session = session

    async def create_post(
        self,
        post_id: int,
        author_id: int,
        title: str,
        tags: list[str]
    ) -> None:
        """Create post and update all views atomically"""
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

        # By tag views - one statement per tag
        for _ in tags:
            batch.append_statement(
                "INSERT INTO posts_by_tag (tag, created, id, title) VALUES (?, ?, ?, ?)"
            )

        # Build values list - one dict per statement in batch
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
from rsylla import Session

class DistributedLock:
    """Distributed lock using LWT"""

    def __init__(self, session: Session, ttl_seconds: int = 30):
        self.session = session
        self.ttl = ttl_seconds
        self._acquire_stmt = None
        self._release_stmt = None

    async def _ensure_prepared(self) -> None:
        if self._acquire_stmt is None:
            # Note: TTL must be embedded in query as it's not a bind parameter
            self._acquire_stmt = await self.session.prepare(
                f"""INSERT INTO locks (name, holder, acquired_at)
                   VALUES (?, ?, ?)
                   IF NOT EXISTS
                   USING TTL {self.ttl}"""
            )
            self._release_stmt = await self.session.prepare(
                "DELETE FROM locks WHERE name = ? IF holder = ?"
            )

    async def acquire(self, lock_name: str, holder: str | None = None) -> str | None:
        """Try to acquire lock. Returns holder ID if acquired, None otherwise."""
        await self._ensure_prepared()

        holder = holder or str(uuid.uuid4())
        now = int(time.time() * 1000)

        result = await self.session.execute_prepared(
            self._acquire_stmt,
            {"name": lock_name, "holder": holder, "acquired_at": now}
        )

        row = result.first_row()
        if row and row[0]:  # [applied] = true
            return holder
        return None

    async def release(self, lock_name: str, holder: str) -> bool:
        """Release lock if we own it. Returns True if released."""
        await self._ensure_prepared()

        result = await self.session.execute_prepared(
            self._release_stmt,
            {"name": lock_name, "holder": holder}
        )
        row = result.first_row()
        return bool(row and row[0])  # [applied]

    async def with_lock(self, lock_name: str, callback):
        """Execute callback with lock held"""
        holder = await self.acquire(lock_name)
        if not holder:
            raise Exception(f"Could not acquire lock: {lock_name}")

        try:
            return await callback()
        finally:
            await self.release(lock_name, holder)


# Usage
async def example():
    session = await Session.connect(["localhost:9042"])
    lock = DistributedLock(session)

    async def critical_section():
        print("Doing exclusive work...")

    await lock.with_lock("my-resource", critical_section)
```

## Read-Through Cache

```python
import pickle
from rsylla import Session

class CachedRepository:
    """Repository with caching layer"""

    def __init__(self, session: Session, cache_ttl: int = 300):
        self.session = session
        self.cache_ttl = cache_ttl
        self._cache_get_stmt = None
        self._cache_set_stmt = None
        self._user_get_stmt = None
        self._cache_del_stmt = None

    async def _ensure_prepared(self) -> None:
        if self._cache_get_stmt is None:
            self._cache_get_stmt = await self.session.prepare(
                "SELECT value FROM cache WHERE key = ?"
            )
            # Note: TTL embedded in query
            self._cache_set_stmt = await self.session.prepare(
                f"INSERT INTO cache (key, value) VALUES (?, ?) USING TTL {self.cache_ttl}"
            )
            self._user_get_stmt = await self.session.prepare(
                "SELECT id, name, email FROM users WHERE id = ?"
            )
            self._cache_del_stmt = await self.session.prepare(
                "DELETE FROM cache WHERE key = ?"
            )

    async def get_user(self, user_id: int) -> dict | None:
        """Get user with caching"""
        await self._ensure_prepared()

        cache_key = f"user:{user_id}"

        # Try cache first
        result = await self.session.execute_prepared(
            self._cache_get_stmt, {"key": cache_key}
        )
        row = result.first_row()
        if row:
            return pickle.loads(row[0])

        # Cache miss - fetch from source
        result = await self.session.execute_prepared(
            self._user_get_stmt, {"id": user_id}
        )
        row = result.first_row()
        if not row:
            return None

        user = {"id": row[0], "name": row[1], "email": row[2]}

        # Store in cache
        await self.session.execute_prepared(
            self._cache_set_stmt,
            {"key": cache_key, "value": pickle.dumps(user)}
        )

        return user

    async def invalidate(self, user_id: int) -> None:
        """Invalidate cache entry"""
        await self._ensure_prepared()

        await self.session.execute_prepared(
            self._cache_del_stmt,
            {"key": f"user:{user_id}"}
        )
```

## Saga Pattern

```python
from rsylla import Session

class OrderSaga:
    """Distributed transaction using saga pattern"""

    def __init__(self, session: Session):
        self.session = session
        self._reserve_stmt = None
        self._release_stmt = None
        self._create_order_stmt = None
        self._cancel_order_stmt = None
        self._confirm_order_stmt = None

    async def _ensure_prepared(self) -> None:
        if self._reserve_stmt is None:
            self._reserve_stmt = await self.session.prepare(
                "UPDATE inventory SET reserved = reserved + ? WHERE id = ? IF quantity >= ?"
            )
            self._release_stmt = await self.session.prepare(
                "UPDATE inventory SET reserved = reserved - ? WHERE id = ?"
            )
            self._create_order_stmt = await self.session.prepare(
                "INSERT INTO orders (id, user_id, items, status) VALUES (?, ?, ?, 'pending')"
            )
            self._cancel_order_stmt = await self.session.prepare(
                "UPDATE orders SET status = 'cancelled' WHERE id = ?"
            )
            self._confirm_order_stmt = await self.session.prepare(
                "UPDATE orders SET status = 'confirmed' WHERE id = ?"
            )

    async def create_order(
        self,
        order_id: str,
        user_id: int,
        items: list[dict]
    ) -> None:
        """Create order with compensating transactions"""
        await self._ensure_prepared()
        reserved_items = []

        try:
            # Step 1: Reserve inventory for each item
            for item in items:
                await self._reserve_inventory(item["id"], item["qty"])
                reserved_items.append(item)

            # Step 2: Create order
            await self._create_order(order_id, user_id, items)

            # Step 3: Confirm order
            await self._confirm_order(order_id)

        except Exception:
            # Compensate: release reserved inventory
            for item in reserved_items:
                try:
                    await self._release_inventory(item["id"], item["qty"])
                except Exception:
                    pass  # Log and continue with other compensations

            # Compensate: cancel order if it was created
            try:
                await self._cancel_order(order_id)
            except Exception:
                pass

            raise

    async def _reserve_inventory(self, item_id: str, qty: int) -> None:
        result = await self.session.execute_prepared(
            self._reserve_stmt,
            {"qty": qty, "id": item_id, "min_qty": qty}
        )
        row = result.first_row()
        if not row or not row[0]:  # [applied] = false
            raise Exception(f"Insufficient inventory for {item_id}")

    async def _release_inventory(self, item_id: str, qty: int) -> None:
        await self.session.execute_prepared(
            self._release_stmt,
            {"qty": qty, "id": item_id}
        )

    async def _create_order(self, order_id: str, user_id: int, items: list) -> None:
        import json
        await self.session.execute_prepared(
            self._create_order_stmt,
            {"id": order_id, "user_id": user_id, "items": json.dumps(items)}
        )

    async def _cancel_order(self, order_id: str) -> None:
        await self.session.execute_prepared(
            self._cancel_order_stmt,
            {"id": order_id}
        )

    async def _confirm_order(self, order_id: str) -> None:
        await self.session.execute_prepared(
            self._confirm_order_stmt,
            {"id": order_id}
        )
```

## Connection Pool Management

```python
import asyncio
from contextlib import asynccontextmanager
from rsylla import Session, SessionBuilder

class SessionPool:
    """Manage session lifecycle for applications"""

    _session: Session | None = None
    _lock: asyncio.Lock = asyncio.Lock()

    @classmethod
    async def get_session(cls) -> Session:
        """Get or create the shared session"""
        if cls._session is None:
            async with cls._lock:
                if cls._session is None:
                    cls._session = await (
                        SessionBuilder()
                        .known_node("localhost:9042")
                        .use_keyspace("myapp", False)
                        .pool_size(4)
                        .compression("lz4")
                        .build()
                    )
        return cls._session

    @classmethod
    async def close(cls) -> None:
        """Close the session (call on application shutdown)"""
        cls._session = None  # Session cleanup is handled by Rust


@asynccontextmanager
async def get_db():
    """Context manager for database access"""
    session = await SessionPool.get_session()
    yield session


# Usage in an async application
async def handle_request():
    async with get_db() as session:
        result = await session.execute("SELECT * FROM users LIMIT 10")
        return list(result)
```
