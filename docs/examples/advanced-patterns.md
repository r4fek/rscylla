# Advanced Usage Patterns

This document demonstrates advanced patterns and real-world use cases for rsylla.

## 1. Multi-Tenant Application

```python
import asyncio
from rsylla import Session, SessionBuilder

class MultiTenantDatabase:
    """Database handler for multi-tenant application"""

    def __init__(self, session: Session):
        self.session = session
        self._tenant_stmts: dict = {}
        self._lock = asyncio.Lock()

    @classmethod
    async def create(cls, nodes: list[str]) -> "MultiTenantDatabase":
        """Factory method to create instance with connected session"""
        session = await (
            SessionBuilder()
            .known_nodes(nodes)
            .pool_size(30)
            .compression("lz4")
            .build()
        )
        return cls(session)

    async def get_tenant_data(self, tenant_id: int, user_id: int):
        """Get data for specific tenant"""
        stmt_key = f"get_user_{tenant_id}"

        async with self._lock:
            if stmt_key not in self._tenant_stmts:
                self._tenant_stmts[stmt_key] = await self.session.prepare(
                    f"SELECT * FROM tenant_{tenant_id}.users WHERE id = ?"
                )

        stmt = self._tenant_stmts[stmt_key]
        result = await self.session.execute_prepared(stmt, {"id": user_id})
        return result.first_row()

    async def create_tenant_schema(self, tenant_id: int) -> None:
        """Create keyspace and tables for new tenant"""
        # Create tenant keyspace
        await self.session.execute(f"""
            CREATE KEYSPACE IF NOT EXISTS tenant_{tenant_id}
            WITH replication = {{
                'class': 'NetworkTopologyStrategy',
                'dc1': 3
            }}
        """)

        await self.session.use_keyspace(f"tenant_{tenant_id}", False)

        # Create tables
        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS users (
                id int PRIMARY KEY,
                name text,
                email text,
                created_at timestamp
            )
        """)

        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS events (
                user_id int,
                event_time timestamp,
                event_type text,
                data map<text, text>,
                PRIMARY KEY (user_id, event_time)
            ) WITH CLUSTERING ORDER BY (event_time DESC)
        """)

        await self.session.await_schema_agreement()


# Usage
async def main():
    db = await MultiTenantDatabase.create(["localhost:9042"])

    # Create tenant
    await db.create_tenant_schema(1001)

    # Access tenant data
    user = await db.get_tenant_data(1001, 123)
    print(user)


asyncio.run(main())
```

## 2. Time Series Data

```python
import asyncio
from rsylla import Session, SessionBuilder, Batch
from datetime import datetime, date, timedelta

class TimeSeriesStore:
    """Store and query time series data efficiently"""

    def __init__(self, session: Session):
        self.session = session
        self._insert_stmt = None
        self._query_stmt = None

    @classmethod
    async def create(cls, session: Session) -> "TimeSeriesStore":
        """Factory method with schema setup"""
        store = cls(session)
        await store._setup_schema()
        return store

    async def _setup_schema(self) -> None:
        """Create optimized time series table"""
        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS metrics (
                metric_name text,
                bucket date,
                timestamp timestamp,
                value double,
                tags map<text, text>,
                PRIMARY KEY ((metric_name, bucket), timestamp)
            ) WITH CLUSTERING ORDER BY (timestamp DESC)
            AND compaction = {
                'class': 'TimeWindowCompactionStrategy',
                'compaction_window_unit': 'DAYS',
                'compaction_window_size': 1
            }
        """)

        # TTL embedded in query since it's not a bind parameter
        self._insert_stmt = await self.session.prepare("""
            INSERT INTO metrics (metric_name, bucket, timestamp, value, tags)
            VALUES (?, ?, ?, ?, ?)
            USING TTL 86400
        """)

        self._query_stmt = await self.session.prepare("""
            SELECT timestamp, value, tags
            FROM metrics
            WHERE metric_name = ?
            AND bucket = ?
            AND timestamp >= ?
            AND timestamp <= ?
        """)

    async def insert_metric(
        self,
        metric_name: str,
        value: float,
        tags: dict | None = None
    ) -> None:
        """Insert a metric data point"""
        now = datetime.utcnow()
        bucket = (now.date() - date(1970, 1, 1)).days
        timestamp = int(now.timestamp() * 1000)

        await self.session.execute_prepared(self._insert_stmt, {
            "metric_name": metric_name,
            "bucket": bucket,
            "timestamp": timestamp,
            "value": value,
            "tags": tags or {}
        })

    async def insert_batch(self, metrics: list[tuple[str, float, dict | None]]) -> None:
        """Efficiently insert multiple metrics"""
        now = datetime.utcnow()
        bucket = (now.date() - date(1970, 1, 1)).days
        timestamp = int(now.timestamp() * 1000)

        batch = Batch("unlogged")
        values = []

        for metric_name, value, tags in metrics:
            batch.append_prepared(self._insert_stmt)
            values.append({
                "metric_name": metric_name,
                "bucket": bucket,
                "timestamp": timestamp,
                "value": value,
                "tags": tags or {}
            })

        await self.session.batch(batch, values)

    async def query_range(
        self,
        metric_name: str,
        start_time: datetime,
        end_time: datetime
    ) -> list[dict]:
        """Query metrics in time range"""
        start_date = start_time.date()
        end_date = end_time.date()
        start_ts = int(start_time.timestamp() * 1000)
        end_ts = int(end_time.timestamp() * 1000)

        all_results = []
        current_date = start_date

        while current_date <= end_date:
            bucket = (current_date - date(1970, 1, 1)).days

            result = await self.session.execute_prepared(self._query_stmt, {
                "metric_name": metric_name,
                "bucket": bucket,
                "start_ts": start_ts,
                "end_ts": end_ts
            })

            for row in result:
                all_results.append({
                    "timestamp": row[0],
                    "value": row[1],
                    "tags": row[2]
                })

            current_date += timedelta(days=1)

        return all_results


# Usage
async def main():
    session = await Session.connect(["localhost:9042"])
    await session.use_keyspace("monitoring", False)

    ts_store = await TimeSeriesStore.create(session)

    # Insert single metric
    await ts_store.insert_metric(
        "cpu.usage", 45.2, {"host": "server1", "region": "us-east"}
    )

    # Insert batch
    metrics = [
        ("cpu.usage", 45.2, {"host": "server1"}),
        ("mem.usage", 78.5, {"host": "server1"}),
        ("disk.usage", 62.1, {"host": "server1"}),
    ]
    await ts_store.insert_batch(metrics)

    # Query time range
    start = datetime.utcnow() - timedelta(hours=1)
    end = datetime.utcnow()
    results = await ts_store.query_range("cpu.usage", start, end)

    for row in results:
        print(f"{row['timestamp']}: {row['value']} - {row['tags']}")


asyncio.run(main())
```

## 3. Event Sourcing

```python
import asyncio
import json
import time
import uuid
from rsylla import Session, SessionBuilder

class EventStore:
    """Event sourcing implementation"""

    def __init__(self, session: Session):
        self.session = session
        self._append_stmt = None
        self._events_stmt = None
        self._snapshot_stmt = None

    @classmethod
    async def create(cls, session: Session) -> "EventStore":
        """Factory method with schema setup"""
        store = cls(session)
        await store._setup_schema()
        return store

    async def _setup_schema(self) -> None:
        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS events (
                aggregate_id uuid,
                version int,
                event_type text,
                event_data text,
                timestamp timestamp,
                PRIMARY KEY (aggregate_id, version)
            ) WITH CLUSTERING ORDER BY (version ASC)
        """)

        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS snapshots (
                aggregate_id uuid PRIMARY KEY,
                version int,
                state text,
                timestamp timestamp
            )
        """)

        self._append_stmt = await self.session.prepare("""
            INSERT INTO events (aggregate_id, version, event_type, event_data, timestamp)
            VALUES (?, ?, ?, ?, ?)
            IF NOT EXISTS
        """)

        self._events_stmt = await self.session.prepare("""
            SELECT version, event_type, event_data, timestamp
            FROM events
            WHERE aggregate_id = ?
            AND version >= ?
        """)

        self._snapshot_stmt = await self.session.prepare("""
            INSERT INTO snapshots (aggregate_id, version, state, timestamp)
            VALUES (?, ?, ?, ?)
        """)

    async def append_event(
        self,
        aggregate_id: str,
        version: int,
        event_type: str,
        event_data: dict
    ) -> bool:
        """Append event with optimistic locking"""
        result = await self.session.execute_prepared(self._append_stmt, {
            "aggregate_id": aggregate_id,
            "version": version,
            "event_type": event_type,
            "event_data": json.dumps(event_data),
            "timestamp": int(time.time() * 1000)
        })

        # Check if event was applied (LWT)
        row = result.first_row()
        if row and row[0]:  # [applied] column
            return True
        else:
            raise Exception(f"Version conflict for {aggregate_id} at version {version}")

    async def get_events(self, aggregate_id: str, from_version: int = 0) -> list[dict]:
        """Get all events for an aggregate"""
        result = await self.session.execute_prepared(self._events_stmt, {
            "aggregate_id": aggregate_id,
            "version": from_version
        })

        events = []
        for row in result:
            events.append({
                "version": row[0],
                "type": row[1],
                "data": json.loads(row[2]),
                "timestamp": row[3]
            })

        return events

    async def rebuild_aggregate(self, aggregate_id: str) -> dict:
        """Rebuild aggregate state from events"""
        events = await self.get_events(aggregate_id)

        # Apply events to rebuild state
        state = {}
        for event in events:
            state = self._apply_event(state, event)

        return state

    def _apply_event(self, state: dict, event: dict) -> dict:
        """Apply event to state (domain-specific)"""
        # Example: account aggregate
        if event["type"] == "AccountCreated":
            state["balance"] = event["data"]["initial_balance"]
            state["owner"] = event["data"]["owner"]
        elif event["type"] == "MoneyDeposited":
            state["balance"] = state.get("balance", 0) + event["data"]["amount"]
        elif event["type"] == "MoneyWithdrawn":
            state["balance"] = state.get("balance", 0) - event["data"]["amount"]

        return state

    async def create_snapshot(
        self,
        aggregate_id: str,
        version: int,
        state: dict
    ) -> None:
        """Create snapshot for faster loading"""
        await self.session.execute_prepared(self._snapshot_stmt, {
            "aggregate_id": aggregate_id,
            "version": version,
            "state": json.dumps(state),
            "timestamp": int(time.time() * 1000)
        })


# Usage
async def main():
    session = await Session.connect(["localhost:9042"])
    await session.use_keyspace("event_store", False)

    store = await EventStore.create(session)

    # Create account
    account_id = str(uuid.uuid4())
    await store.append_event(account_id, 1, "AccountCreated", {
        "owner": "Alice",
        "initial_balance": 1000.0
    })

    # Deposit money
    await store.append_event(account_id, 2, "MoneyDeposited", {
        "amount": 500.0
    })

    # Withdraw money
    await store.append_event(account_id, 3, "MoneyWithdrawn", {
        "amount": 200.0
    })

    # Rebuild state
    state = await store.rebuild_aggregate(account_id)
    print(f"Account balance: {state['balance']}")  # 1300.0


asyncio.run(main())
```

## 4. Materialized Views Pattern

```python
import asyncio
import time
import uuid
from rsylla import Session, SessionBuilder, Batch

class MaterializedViewManager:
    """Manage denormalized views"""

    def __init__(self, session: Session):
        self.session = session

    @classmethod
    async def create(cls, session: Session) -> "MaterializedViewManager":
        """Factory method with schema setup"""
        manager = cls(session)
        await manager._setup_schema()
        return manager

    async def _setup_schema(self) -> None:
        # Main table
        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS posts (
                post_id uuid PRIMARY KEY,
                author_id int,
                title text,
                content text,
                tags set<text>,
                created_at timestamp
            )
        """)

        # View by author
        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS posts_by_author (
                author_id int,
                created_at timestamp,
                post_id uuid,
                title text,
                PRIMARY KEY (author_id, created_at, post_id)
            ) WITH CLUSTERING ORDER BY (created_at DESC)
        """)

        # View by tag
        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS posts_by_tag (
                tag text,
                created_at timestamp,
                post_id uuid,
                title text,
                author_id int,
                PRIMARY KEY (tag, created_at, post_id)
            ) WITH CLUSTERING ORDER BY (created_at DESC)
        """)

    async def create_post(
        self,
        post_id: uuid.UUID,
        author_id: int,
        title: str,
        content: str,
        tags: set[str]
    ) -> None:
        """Create post and update all views atomically"""
        created_at = int(time.time() * 1000)

        # Create batch for atomic updates
        batch = Batch("logged")

        # Insert into main table
        batch.append_statement("""
            INSERT INTO posts (post_id, author_id, title, content, tags, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
        """)

        # Update author view
        batch.append_statement("""
            INSERT INTO posts_by_author (author_id, created_at, post_id, title)
            VALUES (?, ?, ?, ?)
        """)

        # Build values list - one dict per statement
        values = [
            {
                "post_id": post_id,
                "author_id": author_id,
                "title": title,
                "content": content,
                "tags": tags,
                "created_at": created_at
            },
            {
                "author_id": author_id,
                "created_at": created_at,
                "post_id": post_id,
                "title": title
            }
        ]

        # Add statement for each tag
        for tag in tags:
            batch.append_statement("""
                INSERT INTO posts_by_tag (tag, created_at, post_id, title, author_id)
                VALUES (?, ?, ?, ?, ?)
            """)
            values.append({
                "tag": tag,
                "created_at": created_at,
                "post_id": post_id,
                "title": title,
                "author_id": author_id
            })

        # Execute batch
        await self.session.batch(batch, values)

    async def get_posts_by_author(self, author_id: int, limit: int = 10) -> list:
        """Get posts by author"""
        result = await self.session.execute("""
            SELECT post_id, title, created_at
            FROM posts_by_author
            WHERE author_id = ?
            LIMIT ?
        """, {"author_id": author_id, "limit": limit})

        return list(result)

    async def get_posts_by_tag(self, tag: str, limit: int = 10) -> list:
        """Get posts by tag"""
        result = await self.session.execute("""
            SELECT post_id, title, author_id, created_at
            FROM posts_by_tag
            WHERE tag = ?
            LIMIT ?
        """, {"tag": tag, "limit": limit})

        return list(result)


# Usage
async def main():
    session = await Session.connect(["localhost:9042"])
    await session.use_keyspace("blog", False)

    manager = await MaterializedViewManager.create(session)

    # Create post (updates all views atomically)
    post_id = uuid.uuid4()
    await manager.create_post(
        post_id=post_id,
        author_id=123,
        title="My First Post",
        content="This is the content...",
        tags={"python", "database", "scylla"}
    )

    # Query by author
    author_posts = await manager.get_posts_by_author(123)
    for row in author_posts:
        print(f"Post: {row[1]}")

    # Query by tag
    python_posts = await manager.get_posts_by_tag("python")
    for row in python_posts:
        print(f"Post: {row[1]} by {row[2]}")


asyncio.run(main())
```

## 5. Caching Layer

```python
import asyncio
import pickle
import time
from datetime import datetime, timedelta
from rsylla import Session, SessionBuilder

class CacheLayer:
    """Two-level cache with ScyllaDB"""

    def __init__(self, session: Session, memory_ttl_seconds: int = 300):
        self.session = session
        self._memory_cache: dict = {}
        self._memory_ttl = memory_ttl_seconds
        self._get_stmt = None
        self._set_stmt = None
        self._del_stmt = None

    @classmethod
    async def create(
        cls,
        session: Session,
        memory_ttl_seconds: int = 300
    ) -> "CacheLayer":
        """Factory method with schema setup"""
        cache = cls(session, memory_ttl_seconds)
        await cache._setup_schema()
        return cache

    async def _setup_schema(self) -> None:
        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS cache (
                key text PRIMARY KEY,
                value blob,
                created_at timestamp
            )
        """)

        self._get_stmt = await self.session.prepare(
            "SELECT value FROM cache WHERE key = ?"
        )

        # Default TTL of 1 hour
        self._set_stmt = await self.session.prepare(
            "INSERT INTO cache (key, value, created_at) VALUES (?, ?, ?) USING TTL 3600"
        )

        self._del_stmt = await self.session.prepare(
            "DELETE FROM cache WHERE key = ?"
        )

    async def get(self, key: str):
        """Get value from cache (memory -> scylla)"""
        # Check memory cache first
        if key in self._memory_cache:
            value, expiry = self._memory_cache[key]
            if datetime.now() < expiry:
                return value
            else:
                del self._memory_cache[key]

        # Check ScyllaDB
        result = await self.session.execute_prepared(self._get_stmt, {"key": key})
        row = result.first_row()

        if row:
            value = pickle.loads(row[0])
            # Update memory cache
            self._memory_cache[key] = (
                value,
                datetime.now() + timedelta(seconds=self._memory_ttl)
            )
            return value

        return None

    async def set(self, key: str, value, ttl_seconds: int = 3600) -> None:
        """Set value in cache"""
        # Serialize value
        serialized = pickle.dumps(value)

        # Store in ScyllaDB with TTL
        # Note: For dynamic TTL, you'd need separate prepared statements
        await self.session.execute_prepared(self._set_stmt, {
            "key": key,
            "value": serialized,
            "created_at": int(time.time() * 1000)
        })

        # Update memory cache
        memory_ttl = min(ttl_seconds, self._memory_ttl)
        self._memory_cache[key] = (
            value,
            datetime.now() + timedelta(seconds=memory_ttl)
        )

    async def delete(self, key: str) -> None:
        """Delete from cache"""
        await self.session.execute_prepared(self._del_stmt, {"key": key})

        if key in self._memory_cache:
            del self._memory_cache[key]


# Usage
async def main():
    session = await Session.connect(["localhost:9042"])
    await session.use_keyspace("cache", False)

    cache = await CacheLayer.create(session)

    # Set value
    await cache.set(
        "user:123",
        {"name": "Alice", "email": "alice@example.com"},
        ttl_seconds=3600
    )

    # Get value (from memory cache on second call)
    user_data = await cache.get("user:123")
    print(user_data)

    # Delete
    await cache.delete("user:123")


asyncio.run(main())
```

## 6. Rate Limiting

```python
import asyncio
import time
from rsylla import Session, SessionBuilder

class RateLimiter:
    """Token bucket rate limiter using ScyllaDB counters"""

    def __init__(self, session: Session):
        self.session = session
        self._config_get_stmt = None
        self._config_set_stmt = None
        self._count_get_stmt = None
        self._count_incr_stmt = None

    @classmethod
    async def create(cls, session: Session) -> "RateLimiter":
        """Factory method with schema setup"""
        limiter = cls(session)
        await limiter._setup_schema()
        return limiter

    async def _setup_schema(self) -> None:
        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS rate_limits (
                identifier text,
                bucket_time bigint,
                tokens counter,
                PRIMARY KEY (identifier, bucket_time)
            )
        """)

        await self.session.execute("""
            CREATE TABLE IF NOT EXISTS rate_limit_config (
                identifier text PRIMARY KEY,
                max_tokens int,
                refill_rate int,
                bucket_size_seconds int
            )
        """)

        self._config_get_stmt = await self.session.prepare("""
            SELECT max_tokens, bucket_size_seconds
            FROM rate_limit_config
            WHERE identifier = ?
        """)

        self._config_set_stmt = await self.session.prepare("""
            INSERT INTO rate_limit_config
            (identifier, max_tokens, refill_rate, bucket_size_seconds)
            VALUES (?, ?, ?, ?)
        """)

        self._count_get_stmt = await self.session.prepare("""
            SELECT tokens
            FROM rate_limits
            WHERE identifier = ?
            AND bucket_time = ?
        """)

        self._count_incr_stmt = await self.session.prepare("""
            UPDATE rate_limits
            SET tokens = tokens + ?
            WHERE identifier = ?
            AND bucket_time = ?
        """)

    async def configure(
        self,
        identifier: str,
        max_tokens: int,
        refill_rate: int,
        bucket_size_seconds: int = 60
    ) -> None:
        """Configure rate limit for identifier"""
        await self.session.execute_prepared(self._config_set_stmt, {
            "identifier": identifier,
            "max_tokens": max_tokens,
            "refill_rate": refill_rate,
            "bucket_size_seconds": bucket_size_seconds
        })

    async def check_rate_limit(self, identifier: str, tokens_needed: int = 1) -> bool:
        """Check if request is allowed"""
        # Get configuration
        config_result = await self.session.execute_prepared(
            self._config_get_stmt,
            {"identifier": identifier}
        )

        config = config_result.first_row()
        if not config:
            return True  # No limit configured

        max_tokens = config[0]
        bucket_size = config[1]

        # Calculate current bucket
        current_time = int(time.time())
        bucket_time = (current_time // bucket_size) * bucket_size

        # Get current token count
        count_result = await self.session.execute_prepared(self._count_get_stmt, {
            "identifier": identifier,
            "bucket_time": bucket_time
        })

        current_tokens = 0
        row = count_result.first_row()
        if row:
            current_tokens = row[0]

        if current_tokens + tokens_needed <= max_tokens:
            # Allow request and increment counter
            await self.session.execute_prepared(self._count_incr_stmt, {
                "tokens": tokens_needed,
                "identifier": identifier,
                "bucket_time": bucket_time
            })
            return True
        else:
            return False  # Rate limit exceeded


# Usage
async def main():
    session = await Session.connect(["localhost:9042"])
    await session.use_keyspace("rate_limiting", False)

    limiter = await RateLimiter.create(session)

    # Configure: max 100 requests per 60 seconds
    await limiter.configure(
        "api:user:123",
        max_tokens=100,
        refill_rate=100,
        bucket_size_seconds=60
    )

    # Check rate limit
    if await limiter.check_rate_limit("api:user:123"):
        print("Request allowed")
        # Process request
    else:
        print("Rate limit exceeded")
        # Return 429 error


asyncio.run(main())
```

These patterns demonstrate real-world use cases and show how to effectively use rsylla for complex applications.
