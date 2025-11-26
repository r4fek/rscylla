# Results API

The `QueryResult` and `Row` classes represent query results and individual rows.

## QueryResult

`QueryResult` contains the result of a query execution.

### Methods

#### `rows() -> List[Row]`

Get all rows as a list.

```python
result = await session.execute("SELECT * FROM users")
all_rows = result.rows()
for row in all_rows:
    print(row.columns())
```

#### `first_row() -> Optional[Row]`

Get the first row, or `None` if empty.

```python
result = await session.execute("SELECT * FROM users WHERE id = ?", {"id": 1})
row = result.first_row()
if row:
    print(row.columns())
```

#### `single_row() -> Row`

Get the single row. Raises if not exactly one row.

```python
try:
    row = result.single_row()
    print(row.columns())
except ValueError as e:
    print(f"Expected one row: {e}")
```

**Raises:** `ValueError` if zero or multiple rows

#### `first_row_typed() -> Optional[Dict]`

Get the first row as a dictionary.

```python
row_dict = result.first_row_typed()
if row_dict:
    print(row_dict)  # {"col_0": value, "col_1": value, ...}
```

#### `rows_typed() -> List[Dict]`

Get all rows as dictionaries.

```python
rows = result.rows_typed()
for row in rows:
    print(row)
```

#### `col_specs() -> List[Dict]`

Get column specifications.

```python
specs = result.col_specs()
for spec in specs:
    print(f"Column: {spec['name']}, Type: {spec['typ']}")
```

#### `tracing_id() -> Optional[str]`

Get the trace ID if tracing was enabled.

```python
if result.tracing_id():
    print(f"Trace: {result.tracing_id()}")
```

#### `warnings() -> List[str]`

Get any warnings from the query.

```python
for warning in result.warnings():
    print(f"Warning: {warning}")
```

### Special Methods

#### `__iter__`

Iterate over rows.

```python
for row in result:
    print(row.columns())
```

#### `__len__`

Get number of rows.

```python
print(f"Found {len(result)} rows")
```

#### `__bool__`

Check if result has rows.

```python
if result:
    print("Has rows")
else:
    print("Empty result")
```

---

## Row

`Row` represents a single row from query results.

### Methods

#### `columns() -> List[Any]`

Get all column values as a list.

```python
row = result.first_row()
id, name, email = row.columns()
```

#### `as_dict() -> Dict[str, Any]`

Convert row to dictionary.

```python
row_dict = row.as_dict()
# {"col_0": value, "col_1": value, ...}
```

#### `get(index: int) -> Any`

Get column value by index.

```python
name = row.get(1)
```

**Raises:** `IndexError` if index out of range

### Special Methods

#### `__getitem__`

Access columns by index.

```python
id = row[0]
name = row[1]
email = row[2]

# Negative indexing
last = row[-1]
```

#### `__len__`

Get number of columns.

```python
print(f"Row has {len(row)} columns")
```

#### `__repr__`

String representation.

```python
print(row)  # Row(columns=3)
```

---

## Usage Examples

### Check if Exists

```python
result = await session.execute(
    "SELECT id FROM users WHERE id = ?",
    {"id": 123}
)

if result.first_row():
    print("User exists")
else:
    print("User not found")
```

### Get Single Value

```python
result = await session.execute(
    "SELECT COUNT(*) FROM users"
)
count = result.first_row()[0]
print(f"Total users: {count}")
```

### Process All Rows

```python
result = await session.execute("SELECT id, name, email FROM users")

for row in result:
    id, name, email = row.columns()
    print(f"{id}: {name} <{email}>")
```

### Get as Dictionaries

```python
result = await session.execute("SELECT * FROM users")
users = result.rows_typed()

for user in users:
    print(user)  # {"col_0": 1, "col_1": "Alice", ...}
```

### Check Warnings

```python
result = await session.execute("SELECT * FROM large_table")

if result.warnings():
    for warning in result.warnings():
        print(f"Warning: {warning}")
```
