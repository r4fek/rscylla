# Tutorial: Building Your First Application

This tutorial walks you through building a complete application with rsylla, covering all essential concepts.

## What We'll Build

A simple user management system with:

- User CRUD operations
- Session-based authentication tracking
- Activity logging

## Prerequisites

```bash
# Start ScyllaDB
docker run --name scylla -d -p 9042:9042 scylladb/scylla

# Wait for startup (about 30 seconds)
docker logs -f scylla
```

## Step 1: Project Setup

Create a new Python file `user_app.py`:

```python
import asyncio
import time
from rsylla import Session, SessionBuilder, Query, Batch, ScyllaError

# We'll add our code here
```

## Step 2: Database Connection

### Simple Connection

```python
async def connect_simple():
    """Connect using the simple method"""
    session = await Session.connect(["127.0.0.1:9042"])
    return session
```

### Production Connection

```python
async def connect_production():
    """Connect with production settings"""
    session = await (
        SessionBuilder()
        .known_nodes([
            "node1.example.com:9042",
            "node2.example.com:9042",
            "node3.example.com:9042"
        ])
        .user("app_user", "secure_password")
        .pool_size(20)
        .connection_timeout(10000)
        .compression("lz4")
        .tcp_nodelay(True)
        .build()
    )
    return session
```

## Step 3: Schema Setup

Create keyspace and tables:

```python
async def setup_schema(session):
    """Create keyspace and tables"""

    # Create keyspace
    await session.execute("""
        CREATE KEYSPACE IF NOT EXISTS user_app
        WITH replication = {
            'class': 'SimpleStrategy',
            'replication_factor': 1
        }
    """)

    await session.use_keyspace("user_app", case_sensitive=False)

    # Users table
    await session.execute("""
        CREATE TABLE IF NOT EXISTS users (
            user_id int PRIMARY KEY,
            username text,
            email text,
            password_hash text,
            is_active boolean,
            created_at timestamp,
            last_login timestamp
        )
    """)

    # User sessions table
    await session.execute("""
        CREATE TABLE IF NOT EXISTS user_sessions (
            session_id text PRIMARY KEY,
            user_id int,
            created_at timestamp,
            expires_at timestamp,
            ip_address text
        )
    """)

    # Activity log table (time-series style)
    await session.execute("""
        CREATE TABLE IF NOT EXISTS activity_log (
            user_id int,
            activity_date date,
            activity_time timestamp,
            activity_type text,
            details text,
            PRIMARY KEY ((user_id, activity_date), activity_time)
        ) WITH CLUSTERING ORDER BY (activity_time DESC)
    """)

    # Wait for schema to propagate
    await session.await_schema_agreement()
    print("Schema created successfully!")
```

## Step 4: User CRUD Operations

### Create User

```python
async def create_user(session, user_id, username, email, password_hash):
    """Create a new user"""
    current_time = int(time.time() * 1000)

    await session.execute(
        """INSERT INTO users
           (user_id, username, email, password_hash, is_active, created_at)
           VALUES (?, ?, ?, ?, ?, ?)""",
        {
            "user_id": user_id,
            "username": username,
            "email": email,
            "password_hash": password_hash,
            "is_active": True,
            "created_at": current_time
        }
    )

    # Log the activity
    await log_activity(session, user_id, "USER_CREATED", f"User {username} created")

    return user_id
```

### Read User

```python
async def get_user(session, user_id):
    """Get user by ID"""
    result = await session.execute(
        "SELECT * FROM users WHERE user_id = ?",
        {"user_id": user_id}
    )

    row = result.first_row()
    if row:
        columns = row.columns()
        return {
            "user_id": columns[0],
            "username": columns[1],
            "email": columns[2],
            "is_active": columns[4],
            "created_at": columns[5],
            "last_login": columns[6]
        }
    return None


async def get_all_users(session, limit=100):
    """Get all users with limit"""
    query = (
        Query("SELECT user_id, username, email, is_active FROM users")
        .with_page_size(limit)
    )

    result = await session.query(query)

    users = []
    for row in result:
        cols = row.columns()
        users.append({
            "user_id": cols[0],
            "username": cols[1],
            "email": cols[2],
            "is_active": cols[3]
        })

    return users
```

### Update User

```python
async def update_user_email(session, user_id, new_email):
    """Update user email"""
    await session.execute(
        "UPDATE users SET email = ? WHERE user_id = ?",
        {"email": new_email, "user_id": user_id}
    )

    await log_activity(session, user_id, "EMAIL_UPDATED", f"Email changed to {new_email}")


async def update_last_login(session, user_id):
    """Update last login timestamp"""
    current_time = int(time.time() * 1000)

    await session.execute(
        "UPDATE users SET last_login = ? WHERE user_id = ?",
        {"last_login": current_time, "user_id": user_id}
    )
```

### Delete User

```python
async def delete_user(session, user_id):
    """Delete a user"""
    await session.execute(
        "DELETE FROM users WHERE user_id = ?",
        {"user_id": user_id}
    )

    print(f"User {user_id} deleted")
```

## Step 5: Using Prepared Statements

For high-performance operations:

```python
class UserRepository:
    """Repository with prepared statements"""

    def __init__(self, session):
        self.session = session
        self._statements_prepared = False

    async def prepare_statements(self):
        """Prepare statements once"""
        if self._statements_prepared:
            return

        self.insert_user_stmt = await self.session.prepare(
            """INSERT INTO users
               (user_id, username, email, password_hash, is_active, created_at)
               VALUES (?, ?, ?, ?, ?, ?)"""
        )

        self.get_user_stmt = await self.session.prepare(
            "SELECT * FROM users WHERE user_id = ?"
        )
        # Mark as idempotent for safe retries
        self.get_user_stmt = self.get_user_stmt.set_idempotent(True)

        self.update_email_stmt = await self.session.prepare(
            "UPDATE users SET email = ? WHERE user_id = ?"
        )

        self._statements_prepared = True
        print("Statements prepared!")

    async def create_user(self, user_id, username, email, password_hash):
        """Create user using prepared statement"""
        await self.prepare_statements()

        await self.session.execute_prepared(
            self.insert_user_stmt,
            {
                "user_id": user_id,
                "username": username,
                "email": email,
                "password_hash": password_hash,
                "is_active": True,
                "created_at": int(time.time() * 1000)
            }
        )
        return user_id

    async def get_user(self, user_id):
        """Get user using prepared statement"""
        await self.prepare_statements()

        result = await self.session.execute_prepared(
            self.get_user_stmt,
            {"user_id": user_id}
        )

        return result.first_row()

    async def bulk_create_users(self, users_data):
        """Create multiple users efficiently"""
        await self.prepare_statements()

        for user in users_data:
            await self.session.execute_prepared(self.insert_user_stmt, user)

        print(f"Created {len(users_data)} users")
```

## Step 6: Activity Logging

Log user activities:

```python
async def log_activity(session, user_id, activity_type, details):
    """Log user activity"""
    from datetime import date

    current_time = int(time.time() * 1000)
    today = (date.today() - date(1970, 1, 1)).days

    await session.execute(
        """INSERT INTO activity_log
           (user_id, activity_date, activity_time, activity_type, details)
           VALUES (?, ?, ?, ?, ?)""",
        {
            "user_id": user_id,
            "activity_date": today,
            "activity_time": current_time,
            "activity_type": activity_type,
            "details": details
        }
    )


async def get_user_activity(session, user_id, activity_date):
    """Get user activity for a specific day"""
    result = await session.execute(
        """SELECT activity_time, activity_type, details
           FROM activity_log
           WHERE user_id = ? AND activity_date = ?""",
        {"user_id": user_id, "activity_date": activity_date}
    )

    activities = []
    for row in result:
        cols = row.columns()
        activities.append({
            "time": cols[0],
            "type": cols[1],
            "details": cols[2]
        })

    return activities
```

## Step 7: Batch Operations

Use batches for atomic operations:

```python
async def create_user_with_session(session, user_data, session_data):
    """Create user and session atomically"""

    batch = Batch("logged")  # Atomic batch

    # Add user insert
    batch.append_statement(
        """INSERT INTO users
           (user_id, username, email, password_hash, is_active, created_at)
           VALUES (?, ?, ?, ?, ?, ?)"""
    )

    # Add session insert
    batch.append_statement(
        """INSERT INTO user_sessions
           (session_id, user_id, created_at, expires_at, ip_address)
           VALUES (?, ?, ?, ?, ?)"""
    )

    values = [user_data, session_data]

    await session.batch(batch, values)
    print("User and session created atomically!")
```

## Step 8: Error Handling

Properly handle errors:

```python
async def safe_get_user(session, user_id):
    """Get user with error handling"""
    try:
        result = await session.execute(
            "SELECT * FROM users WHERE user_id = ?",
            {"user_id": user_id}
        )
        return result.first_row()
    except ScyllaError as e:
        print(f"Database error: {e}")
        return None


async def execute_with_retry(session, query, values, max_retries=3):
    """Execute query with retry logic"""
    for attempt in range(max_retries):
        try:
            return await session.execute(query, values)
        except ScyllaError as e:
            if attempt == max_retries - 1:
                raise
            print(f"Retry {attempt + 1}/{max_retries}: {e}")
            await asyncio.sleep(2 ** attempt)  # Exponential backoff
```

## Step 9: Complete Application

Put it all together:

```python
import asyncio
import time
import uuid
from datetime import date
from rsylla import Session, SessionBuilder, Query, Batch, ScyllaError


async def main():
    print("=" * 50)
    print("rsylla User Management Tutorial")
    print("=" * 50)

    # Connect
    print("\n1. Connecting to ScyllaDB...")
    session = await Session.connect(["127.0.0.1:9042"])
    print("   Connected!")

    # Setup schema
    print("\n2. Setting up schema...")
    await setup_schema(session)

    # Create repository
    print("\n3. Preparing statements...")
    repo = UserRepository(session)
    await repo.prepare_statements()

    # Create users
    print("\n4. Creating users...")
    await repo.create_user(1, "alice", "alice@example.com", "hash123")
    await repo.create_user(2, "bob", "bob@example.com", "hash456")
    await repo.create_user(3, "charlie", "charlie@example.com", "hash789")
    print("   Created 3 users!")

    # Query users
    print("\n5. Querying users...")
    users = await get_all_users(session)
    for user in users:
        print(f"   - {user['username']} ({user['email']})")

    # Update user
    print("\n6. Updating user email...")
    await update_user_email(session, 1, "alice.new@example.com")

    user = await get_user(session, 1)
    print(f"   Updated: {user['username']} -> {user['email']}")

    # Log activity
    print("\n7. Logging activity...")
    await log_activity(session, 1, "LOGIN", "User logged in from 192.168.1.1")
    await log_activity(session, 1, "VIEW_PAGE", "Viewed dashboard")

    # Get activity
    today = (date.today() - date(1970, 1, 1)).days
    activities = await get_user_activity(session, 1, today)
    print(f"   Found {len(activities)} activities")

    # Batch operation
    print("\n8. Creating user with session (batch)...")
    user_data = {
        "user_id": 100,
        "username": "newuser",
        "email": "new@example.com",
        "password_hash": "newhash",
        "is_active": True,
        "created_at": int(time.time() * 1000)
    }
    session_data = {
        "session_id": str(uuid.uuid4()),
        "user_id": 100,
        "created_at": int(time.time() * 1000),
        "expires_at": int(time.time() * 1000) + 3600000,
        "ip_address": "192.168.1.100"
    }
    await create_user_with_session(session, user_data, session_data)

    # Delete user
    print("\n9. Deleting user...")
    await delete_user(session, 3)

    # Final count
    print("\n10. Final user count...")
    users = await get_all_users(session)
    print(f"    Total users: {len(users)}")

    print("\n" + "=" * 50)
    print("Tutorial completed!")
    print("=" * 50)


if __name__ == "__main__":
    asyncio.run(main())
```

## Running the Application

```bash
python user_app.py
```

Expected output:

```
==================================================
rsylla User Management Tutorial
==================================================

1. Connecting to ScyllaDB...
   Connected!

2. Setting up schema...
Schema created successfully!

3. Preparing statements...
Statements prepared!

4. Creating users...
   Created 3 users!

5. Querying users...
   - alice (alice@example.com)
   - bob (bob@example.com)
   - charlie (charlie@example.com)

...

==================================================
Tutorial completed!
==================================================
```

## Key Takeaways

1. **Use `await`** - All rsylla operations are async
2. **Prepare statements** - For repeated queries, prepare once, execute many
3. **Use batches** - For atomic multi-statement operations
4. **Handle errors** - Wrap operations in try/except with ScyllaError
5. **Design for queries** - Create tables that support your query patterns

## Next Steps

- [Data Types Guide](../guide/data-types.md) - Working with CQL types
- [Best Practices](../guide/best-practices.md) - Production patterns
- [API Reference](../api/overview.md) - Complete API documentation
- [Advanced Patterns](../guide/advanced-patterns.md) - Real-world use cases
