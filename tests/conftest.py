"""
Pytest configuration and fixtures for rsylla tests
"""

import os
import sys
import time

import pytest

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

try:
    from rsylla import Session

    RSYLLA_AVAILABLE = True
except ImportError:
    RSYLLA_AVAILABLE = False


def pytest_configure(config):
    """Configure pytest"""
    config.addinivalue_line("markers", "unit: Unit tests")
    config.addinivalue_line("markers", "integration: Integration tests")
    config.addinivalue_line("markers", "slow: Slow tests")


@pytest.fixture(scope="session")
def scylla_host():
    """ScyllaDB host for testing"""
    return os.getenv("SCYLLA_HOST", "127.0.0.1")


@pytest.fixture(scope="session")
def scylla_port():
    """ScyllaDB port for testing"""
    return int(os.getenv("SCYLLA_PORT", "9042"))


@pytest.fixture(scope="session")
def scylla_connection_string(scylla_host, scylla_port):
    """ScyllaDB connection string"""
    return f"{scylla_host}:{scylla_port}"


@pytest.fixture(scope="session")
async def wait_for_scylla(scylla_connection_string):
    """Wait for ScyllaDB to be ready"""
    if not RSYLLA_AVAILABLE:
        pytest.skip("rsylla not available - run 'maturin develop' first")

    max_retries = 60
    retry_delay = 2

    for attempt in range(max_retries):
        try:
            session = await Session.connect([scylla_connection_string])
            # Try a simple query
            await session.execute("SELECT now() FROM system.local")
            print(f"\nScyllaDB is ready at {scylla_connection_string}")
            return True
        except Exception as e:
            if attempt < max_retries - 1:
                print(f"\nWaiting for ScyllaDB... (attempt {attempt + 1}/{max_retries}): {e}")
                time.sleep(retry_delay)
            else:
                pytest.fail(
                    f"ScyllaDB not available after {max_retries} attempts at {scylla_connection_string}"
                )

    return False


@pytest.fixture(scope="session")
async def session(scylla_connection_string, wait_for_scylla):
    """Create a session for testing"""
    session = await Session.connect([scylla_connection_string])
    yield session
    # Session cleanup happens automatically


@pytest.fixture(scope="function")
async def test_keyspace(session):
    """Create and use a test keyspace"""
    keyspace_name = "test_rsylla"

    # Create keyspace
    await session.execute(
        f"""
        CREATE KEYSPACE IF NOT EXISTS {keyspace_name}
        WITH replication = {{'class': 'SimpleStrategy', 'replication_factor': 1}}
    """
    )

    # Use keyspace
    await session.use_keyspace(keyspace_name, False)

    # Wait for schema agreement
    await session.await_schema_agreement()

    yield keyspace_name

    # Cleanup: drop keyspace
    try:
        await session.execute(f"DROP KEYSPACE IF EXISTS {keyspace_name}")
    except:
        pass


@pytest.fixture(scope="function")
async def users_table(session, test_keyspace):
    """Create a users table for testing"""
    await session.execute(
        """
        CREATE TABLE IF NOT EXISTS users (
            id int PRIMARY KEY,
            username text,
            email text,
            age int,
            is_active boolean,
            created_at timestamp
        )
    """
    )

    await session.await_schema_agreement()

    yield "users"

    # Cleanup
    try:
        await session.execute("DROP TABLE IF EXISTS users")
    except:
        pass


@pytest.fixture(scope="function")
async def sample_users(session, users_table):
    """Insert sample users for testing"""
    import time

    users = [
        {
            "id": 1,
            "username": "alice",
            "email": "alice@example.com",
            "age": 30,
            "is_active": True,
            "created_at": int(time.time() * 1000),
        },
        {
            "id": 2,
            "username": "bob",
            "email": "bob@example.com",
            "age": 25,
            "is_active": True,
            "created_at": int(time.time() * 1000),
        },
        {
            "id": 3,
            "username": "charlie",
            "email": "charlie@example.com",
            "age": 35,
            "is_active": False,
            "created_at": int(time.time() * 1000),
        },
    ]

    for user in users:
        await session.execute(
            "INSERT INTO users (id, username, email, age, is_active, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            user,
        )

    yield users

    # Cleanup happens when table is dropped
