# Real-World Patterns

This page demonstrates real-world usage patterns and architectures.

## User Session Management

```python
import uuid
import time
from rsylla import Session, Batch

class SessionManager:
    """Manage user sessions with expiration"""

    def __init__(self, session, ttl_seconds=3600):
        self.session = session
        self.ttl = ttl_seconds

    async def create_session(self, user_id, ip_address):
        """Create new session for user"""
        session_id = str(uuid.uuid4())
        now = int(time.time() * 1000)

        await self.session.execute(
            """INSERT INTO sessions (id, user_id, ip, created_at)
               VALUES (?, ?, ?, ?) USING TTL ?""",
            {
                "id": session_id,
                "user_id": user_id,
                "ip": ip_address,
                "created_at": now,
                "ttl": self.ttl
            }
        )
        return session_id

    async def validate_session(self, session_id):
        """Check if session is valid"""
        result = await self.session.execute(
            "SELECT user_id FROM sessions WHERE id = ?",
            {"id": session_id}
        )
        row = result.first_row()
        return row[0] if row else None

    async def destroy_session(self, session_id):
        """Delete session"""
        await self.session.execute(
            "DELETE FROM sessions WHERE id = ?",
            {"id": session_id}
        )
```

## Time Series Metrics

```python
from datetime import datetime, date

class MetricsStore:
    """Store and query time series metrics"""

    def __init__(self, session):
        self.session = session

    async def record(self, metric, value, tags=None):
        """Record a metric value"""
        now = datetime.utcnow()
        bucket = (now.date() - date(1970, 1, 1)).days

        await self.session.execute(
            """INSERT INTO metrics (name, bucket, ts, value, tags)
               VALUES (?, ?, ?, ?, ?) USING TTL 604800""",
            {
                "name": metric,
                "bucket": bucket,
                "ts": int(now.timestamp() * 1000),
                "value": value,
                "tags": tags or {}
            }
        )

    async def query(self, metric, start_time, end_time):
        """Query metrics in time range"""
        start_bucket = (start_time.date() - date(1970, 1, 1)).days
        end_bucket = (end_time.date() - date(1970, 1, 1)).days

        results = []
        for bucket in range(start_bucket, end_bucket + 1):
            result = await self.session.execute(
                """SELECT ts, value FROM metrics
                   WHERE name = ? AND bucket = ?
                   AND ts >= ? AND ts <= ?""",
                {
                    "name": metric,
                    "bucket": bucket,
                    "ts_start": int(start_time.timestamp() * 1000),
                    "ts_end": int(end_time.timestamp() * 1000)
                }
            )
            results.extend(result.rows())

        return results
```

## Rate Limiter

```python
class RateLimiter:
    """Token bucket rate limiter using counters"""

    def __init__(self, session):
        self.session = session

    async def check(self, key, limit, window_seconds=60):
        """Check if request is allowed"""
        window = int(time.time()) // window_seconds

        # Get current count
        result = await self.session.execute(
            "SELECT count FROM rate_limits WHERE key = ? AND window = ?",
            {"key": key, "window": window}
        )

        row = result.first_row()
        current = row[0] if row else 0

        if current >= limit:
            return False

        # Increment counter
        await self.session.execute(
            """UPDATE rate_limits SET count = count + 1
               WHERE key = ? AND window = ?""",
            {"key": key, "window": window}
        )

        return True
```

## Cache Layer

```python
import pickle

class Cache:
    """Simple cache using ScyllaDB"""

    def __init__(self, session, default_ttl=3600):
        self.session = session
        self.default_ttl = default_ttl

    async def get(self, key):
        """Get value from cache"""
        result = await self.session.execute(
            "SELECT value FROM cache WHERE key = ?",
            {"key": key}
        )
        row = result.first_row()
        if row:
            return pickle.loads(row[0])
        return None

    async def set(self, key, value, ttl=None):
        """Set value in cache"""
        ttl = ttl or self.default_ttl
        await self.session.execute(
            "INSERT INTO cache (key, value) VALUES (?, ?) USING TTL ?",
            {"key": key, "value": pickle.dumps(value), "ttl": ttl}
        )

    async def delete(self, key):
        """Delete from cache"""
        await self.session.execute(
            "DELETE FROM cache WHERE key = ?",
            {"key": key}
        )

    async def get_or_set(self, key, factory, ttl=None):
        """Get from cache or compute and store"""
        value = await self.get(key)
        if value is None:
            value = await factory()
            await self.set(key, value, ttl)
        return value
```

## Event Sourcing

```python
import json

class EventStore:
    """Simple event store implementation"""

    def __init__(self, session):
        self.session = session

    async def append(self, stream_id, event_type, data, expected_version=None):
        """Append event to stream"""
        # Get current version
        result = await self.session.execute(
            "SELECT MAX(version) FROM events WHERE stream_id = ?",
            {"stream_id": stream_id}
        )
        row = result.first_row()
        current_version = row[0] if row and row[0] else 0

        if expected_version is not None and current_version != expected_version:
            raise Exception("Concurrency conflict")

        new_version = current_version + 1

        await self.session.execute(
            """INSERT INTO events (stream_id, version, type, data, ts)
               VALUES (?, ?, ?, ?, ?)""",
            {
                "stream_id": stream_id,
                "version": new_version,
                "type": event_type,
                "data": json.dumps(data),
                "ts": int(time.time() * 1000)
            }
        )

        return new_version

    async def read(self, stream_id, from_version=0):
        """Read events from stream"""
        result = await self.session.execute(
            """SELECT version, type, data FROM events
               WHERE stream_id = ? AND version > ?
               ORDER BY version""",
            {"stream_id": stream_id, "version": from_version}
        )

        events = []
        for row in result:
            events.append({
                "version": row[0],
                "type": row[1],
                "data": json.loads(row[2])
            })
        return events
```

## Repository Pattern

```python
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class User:
    id: int
    name: str
    email: str
    created_at: int

class UserRepository:
    """Complete user repository"""

    def __init__(self, session):
        self.session = session
        self._stmts = {}

    async def _prepare(self):
        if self._stmts:
            return

        self._stmts["insert"] = await self.session.prepare(
            "INSERT INTO users (id, name, email, created_at) VALUES (?, ?, ?, ?)"
        )
        self._stmts["get"] = await self.session.prepare(
            "SELECT * FROM users WHERE id = ?"
        )
        self._stmts["update"] = await self.session.prepare(
            "UPDATE users SET name = ?, email = ? WHERE id = ?"
        )
        self._stmts["delete"] = await self.session.prepare(
            "DELETE FROM users WHERE id = ?"
        )

    async def create(self, user: User) -> None:
        await self._prepare()
        await self.session.execute_prepared(self._stmts["insert"], {
            "id": user.id,
            "name": user.name,
            "email": user.email,
            "created_at": user.created_at
        })

    async def get(self, user_id: int) -> Optional[User]:
        await self._prepare()
        result = await self.session.execute_prepared(
            self._stmts["get"], {"id": user_id}
        )
        row = result.first_row()
        if row:
            cols = row.columns()
            return User(id=cols[0], name=cols[1], email=cols[2], created_at=cols[3])
        return None

    async def update(self, user: User) -> None:
        await self._prepare()
        await self.session.execute_prepared(self._stmts["update"], {
            "name": user.name,
            "email": user.email,
            "id": user.id
        })

    async def delete(self, user_id: int) -> None:
        await self._prepare()
        await self.session.execute_prepared(
            self._stmts["delete"], {"id": user_id}
        )
```

## Database Connection Manager

```python
class DatabaseManager:
    """Manage database connection lifecycle"""

    _instance = None

    def __init__(self):
        self._session = None

    @classmethod
    def instance(cls):
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    async def connect(self, nodes, keyspace, **kwargs):
        """Initialize connection"""
        from rsylla import SessionBuilder

        builder = SessionBuilder().known_nodes(nodes)

        if kwargs.get("user"):
            builder = builder.user(kwargs["user"], kwargs["password"])
        if kwargs.get("pool_size"):
            builder = builder.pool_size(kwargs["pool_size"])
        if kwargs.get("compression"):
            builder = builder.compression(kwargs["compression"])

        self._session = await builder.build()

        if keyspace:
            await self._session.use_keyspace(keyspace, False)

    @property
    def session(self):
        if not self._session:
            raise RuntimeError("Database not connected")
        return self._session

# Usage
db = DatabaseManager.instance()
await db.connect(["node1:9042"], "myapp", pool_size=20)
result = await db.session.execute("SELECT * FROM users")
```
