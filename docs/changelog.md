# Changelog

All notable changes to rsylla are documented here.

## [0.1.1] - 2024

### Added
- Comprehensive benchmark suite
- Updated documentation
- Performance comparisons with other drivers

### Improved
- Documentation and examples
- Type hints coverage

## [0.1.0] - 2024

### Initial Release

#### Core Features
- Session management with connection pooling
- Query execution with parameter binding
- Prepared statements for optimal performance
- Batch operations (logged, unlogged, counter)
- Full consistency level support
- Query tracing
- Compression (LZ4, Snappy)
- Authentication support

#### API
- `Session` - Main database interface
- `SessionBuilder` - Fluent session configuration
- `Query` - Configurable query execution
- `PreparedStatement` - Pre-compiled statements
- `Batch` - Batch operations
- `QueryResult` - Result iteration and access
- `Row` - Row data access
- `ScyllaError` - Error handling

#### Data Types
- All CQL basic types (int, bigint, text, boolean, etc.)
- Collection types (list, set, map)
- UUID and TimeUUID
- Timestamp, Date, Time
- Blob and binary data
- User Defined Types
- Counters

#### Performance
- ~3.9x faster than cassandra-driver
- ~1.2x faster than acsylla
- 85,000+ ops/sec throughput
- Sub-millisecond latencies

---

## Versioning

rsylla follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version for incompatible API changes
- **MINOR** version for new functionality (backwards compatible)
- **PATCH** version for bug fixes (backwards compatible)

## Upgrading

### From 0.1.0 to 0.1.1

No breaking changes. Simply upgrade:

```bash
pip install --upgrade rsylla
```
