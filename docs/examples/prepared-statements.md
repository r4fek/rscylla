# Prepared Statements Examples

Prepared statements are pre-compiled queries that provide better performance for repeated executions.

## Basic Usage

```python
from rsylla import Session

async def main():
    session = await Session.connect(["127.0.0.1:9042"])
    await session.use_keyspace("example", False)

    # Prepare the statement
    insert_stmt = await session.prepare(
        "INSERT INTO users (id, name, email) VALUES (?, ?, ?)"
    )

    # Execute multiple times
    for i in range(1000):
        await session.execute_prepared(insert_stmt, {
            "id": i,
            "name": f"User {i}",
            "email": f"user{i}@example.com"
        })
```

## With Configuration

```python
# Prepare with consistency level
select_stmt = await session.prepare(
    "SELECT * FROM users WHERE id = ?"
)
select_stmt = (
    select_stmt
    .with_consistency("LOCAL_QUORUM")
    .set_idempotent(True)
)

# Execute
result = await session.execute_prepared(select_stmt, {"id": 1})
```

## Statement Repository Pattern

```python
class UserRepository:
    """Repository managing prepared statements"""

    def __init__(self, session):
        self.session = session
        self._prepared = False

    async def _ensure_prepared(self):
        if self._prepared:
            return

        self.insert_stmt = await self.session.prepare(
            "INSERT INTO users (id, name, email, created_at) VALUES (?, ?, ?, ?)"
        )

        self.get_stmt = await self.session.prepare(
            "SELECT * FROM users WHERE id = ?"
        )
        self.get_stmt = self.get_stmt.set_idempotent(True)

        self.update_email_stmt = await self.session.prepare(
            "UPDATE users SET email = ? WHERE id = ?"
        )

        self.delete_stmt = await self.session.prepare(
            "DELETE FROM users WHERE id = ?"
        )

        self._prepared = True

    async def create(self, user_id, name, email):
        await self._ensure_prepared()
        import time
        await self.session.execute_prepared(self.insert_stmt, {
            "id": user_id,
            "name": name,
            "email": email,
            "created_at": int(time.time() * 1000)
        })

    async def get(self, user_id):
        await self._ensure_prepared()
        result = await self.session.execute_prepared(
            self.get_stmt, {"id": user_id}
        )
        return result.first_row()

    async def update_email(self, user_id, new_email):
        await self._ensure_prepared()
        await self.session.execute_prepared(self.update_email_stmt, {
            "email": new_email, "id": user_id
        })

    async def delete(self, user_id):
        await self._ensure_prepared()
        await self.session.execute_prepared(
            self.delete_stmt, {"id": user_id}
        )

# Usage
repo = UserRepository(session)
await repo.create(1, "Alice", "alice@example.com")
user = await repo.get(1)
```

## Bulk Operations

```python
async def bulk_insert(session, users):
    """Insert many users efficiently"""
    insert_stmt = await session.prepare(
        "INSERT INTO users (id, name, email) VALUES (?, ?, ?)"
    )

    for user in users:
        await session.execute_prepared(insert_stmt, user)

# Usage
users = [
    {"id": i, "name": f"User {i}", "email": f"user{i}@example.com"}
    for i in range(10000)
]
await bulk_insert(session, users)
```

## Performance Comparison

```python
import time

async def benchmark():
    session = await Session.connect(["127.0.0.1:9042"])

    # Without prepared statement
    start = time.time()
    for i in range(1000):
        await session.execute(
            "SELECT * FROM users WHERE id = ?",
            {"id": i % 100}
        )
    unprepared_time = time.time() - start

    # With prepared statement
    select_stmt = await session.prepare(
        "SELECT * FROM users WHERE id = ?"
    )

    start = time.time()
    for i in range(1000):
        await session.execute_prepared(
            select_stmt,
            {"id": i % 100}
        )
    prepared_time = time.time() - start

    print(f"Unprepared: {unprepared_time:.2f}s")
    print(f"Prepared: {prepared_time:.2f}s")
    print(f"Speedup: {unprepared_time/prepared_time:.2f}x")
```

## Getting Statement Info

```python
prepared = await session.prepare("SELECT * FROM users WHERE id = ?")

# Get statement ID
stmt_id = prepared.get_id()
print(f"ID: {stmt_id.hex()}")

# Get original query
query = prepared.get_statement()
print(f"Query: {query}")

# Check idempotency
print(f"Idempotent: {prepared.is_idempotent()}")
```

## Best Practices

1. **Prepare once, execute many** - Don't re-prepare the same statement
2. **Mark idempotent statements** - Enable safe retries
3. **Use for repeated queries** - Not worth it for one-off queries
4. **Cache prepared statements** - Use a repository or manager class
5. **Set appropriate consistency** - Configure at prepare time
