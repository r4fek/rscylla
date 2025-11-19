use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use scylla::value::CqlValue;
use std::collections::HashMap;

pub fn cql_value_to_py(py: Python, value: &CqlValue) -> PyResult<PyObject> {
    match value {
        CqlValue::Ascii(s) | CqlValue::Text(s) => Ok(s.to_object(py)),
        CqlValue::Boolean(b) => Ok(b.to_object(py)),
        CqlValue::Int(i) => Ok(i.to_object(py)),
        CqlValue::BigInt(i) => Ok(i.to_object(py)),
        CqlValue::SmallInt(i) => Ok(i.to_object(py)),
        CqlValue::TinyInt(i) => Ok(i.to_object(py)),
        CqlValue::Counter(c) => Ok(c.0.to_object(py)),
        CqlValue::Float(f) => Ok(f.to_object(py)),
        CqlValue::Double(d) => Ok(d.to_object(py)),
        CqlValue::Blob(b) => Ok(PyBytes::new_bound(py, b).to_object(py)),
        CqlValue::Uuid(u) => Ok(u.to_string().to_object(py)),
        CqlValue::Timeuuid(t) => Ok(t.to_string().to_object(py)),
        CqlValue::Inet(addr) => Ok(addr.to_string().to_object(py)),
        CqlValue::List(list) => {
            let py_list = PyList::empty_bound(py);
            for item in list {
                py_list.append(cql_value_to_py(py, item)?)?;
            }
            Ok(py_list.to_object(py))
        }
        CqlValue::Set(set) => {
            let py_list = PyList::empty_bound(py);
            for item in set {
                py_list.append(cql_value_to_py(py, item)?)?;
            }
            Ok(py_list.to_object(py))
        }
        CqlValue::Map(map) => {
            let py_dict = PyDict::new_bound(py);
            for (key, val) in map {
                py_dict.set_item(cql_value_to_py(py, key)?, cql_value_to_py(py, val)?)?;
            }
            Ok(py_dict.to_object(py))
        }
        CqlValue::Timestamp(ts) => Ok(ts.0.to_object(py)),
        CqlValue::Date(d) => Ok(d.0.to_object(py)),
        CqlValue::Time(t) => Ok(t.0.to_object(py)),
        CqlValue::Duration(d) => {
            let dict = PyDict::new_bound(py);
            dict.set_item("months", d.months)?;
            dict.set_item("days", d.days)?;
            dict.set_item("nanoseconds", d.nanoseconds)?;
            Ok(dict.to_object(py))
        }
        CqlValue::Varint(v) => {
            // CqlVarint - use Debug representation since fields are private
            Ok(format!("{:?}", v).to_object(py))
        }
        CqlValue::Decimal(d) => {
            // CqlDecimal - use Debug representation since fields are private
            Ok(format!("{:?}", d).to_object(py))
        }
        CqlValue::Tuple(tuple) => {
            let py_list = PyList::empty_bound(py);
            for item in tuple {
                if let Some(val) = item {
                    py_list.append(cql_value_to_py(py, val)?)?;
                } else {
                    py_list.append(py.None())?;
                }
            }
            Ok(py_list.to_object(py))
        }
        CqlValue::UserDefinedType { fields, .. } => {
            let py_dict = PyDict::new_bound(py);
            for (name, value) in fields {
                if let Some(val) = value {
                    py_dict.set_item(name, cql_value_to_py(py, val)?)?;
                } else {
                    py_dict.set_item(name, py.None())?;
                }
            }
            Ok(py_dict.to_object(py))
        }
        CqlValue::Empty => Ok(py.None()),
        _ => {
            // Handle any additional variants that may be added in the future
            Ok(format!("{:?}", value).to_object(py))
        }
    }
}

#[allow(dead_code)]
pub fn py_to_cql_value(obj: &Bound<'_, PyAny>) -> PyResult<CqlValue> {
    if obj.is_none() {
        return Ok(CqlValue::Empty);
    }

    if let Ok(b) = obj.extract::<bool>() {
        return Ok(CqlValue::Boolean(b));
    }

    if let Ok(i) = obj.extract::<i32>() {
        return Ok(CqlValue::Int(i));
    }

    if let Ok(i) = obj.extract::<i64>() {
        return Ok(CqlValue::BigInt(i));
    }

    if let Ok(f) = obj.extract::<f32>() {
        return Ok(CqlValue::Float(f));
    }

    if let Ok(f) = obj.extract::<f64>() {
        return Ok(CqlValue::Double(f));
    }

    if let Ok(s) = obj.extract::<String>() {
        return Ok(CqlValue::Text(s));
    }

    if let Ok(b) = obj.extract::<Vec<u8>>() {
        return Ok(CqlValue::Blob(b));
    }

    if let Ok(list) = obj.downcast::<PyList>() {
        let mut values = Vec::new();
        for item in list.iter() {
            values.push(py_to_cql_value(&item)?);
        }
        return Ok(CqlValue::List(values));
    }

    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = Vec::new();
        for (key, val) in dict.iter() {
            map.push((py_to_cql_value(&key)?, py_to_cql_value(&val)?));
        }
        return Ok(CqlValue::Map(map));
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
        "Cannot convert Python type {:?} to CQL value",
        obj.get_type()
    )))
}

#[allow(dead_code)]
pub fn py_dict_to_values(dict: Option<&Bound<'_, PyDict>>) -> PyResult<HashMap<String, CqlValue>> {
    let mut values = HashMap::new();

    if let Some(d) = dict {
        for (key, val) in d.iter() {
            let key_str = key.extract::<String>()?;
            values.insert(key_str, py_to_cql_value(&val)?);
        }
    }

    Ok(values)
}

// Helper type that can hold different value types for serialization
#[derive(Debug, Clone)]
pub enum SerializableValue {
    Null,
    Bool(bool),
    Int(i32),
    BigInt(i64),
    Float(f32),
    Double(f64),
    Text(String),
    Blob(Vec<u8>),
    Timestamp(chrono::DateTime<chrono::Utc>),
    List(Vec<SerializableValue>),
    #[allow(dead_code)]
    Set(Vec<SerializableValue>),
    // For maps, we use simpler types that scylla can handle directly
    TextMap(HashMap<String, String>),
    IntMap(HashMap<String, i64>),
}

impl scylla::serialize::value::SerializeValue for SerializableValue {
    fn serialize<'b>(
        &self,
        _typ: &scylla::frame::response::result::ColumnType,
        writer: scylla::serialize::writers::CellWriter<'b>,
    ) -> Result<
        scylla::serialize::writers::WrittenCellProof<'b>,
        scylla::serialize::SerializationError,
    > {
        match self {
            SerializableValue::Null => {
                <Option<i32> as scylla::serialize::value::SerializeValue>::serialize(
                    &None, _typ, writer,
                )
            }
            SerializableValue::Bool(b) => b.serialize(_typ, writer),
            SerializableValue::Int(i) => i.serialize(_typ, writer),
            SerializableValue::BigInt(i) => i.serialize(_typ, writer),
            SerializableValue::Float(f) => f.serialize(_typ, writer),
            SerializableValue::Double(f) => f.serialize(_typ, writer),
            SerializableValue::Text(s) => s.serialize(_typ, writer),
            SerializableValue::Blob(b) => b.serialize(_typ, writer),
            SerializableValue::Timestamp(dt) => {
                // Convert to CqlTimestamp (milliseconds since epoch)
                let timestamp = scylla::value::CqlTimestamp(dt.timestamp_millis());
                timestamp.serialize(_typ, writer)
            }
            SerializableValue::List(items) => items.serialize(_typ, writer),
            SerializableValue::Set(items) => {
                // Sets are serialized as lists in scylla
                items.serialize(_typ, writer)
            }
            SerializableValue::TextMap(map) => map.serialize(_typ, writer),
            SerializableValue::IntMap(map) => map.serialize(_typ, writer),
        }
    }
}

pub fn py_dict_to_serialized_values(
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<HashMap<String, SerializableValue>> {
    let mut serialized = HashMap::new();

    if let Some(d) = dict {
        for (key, val) in d.iter() {
            let key_str = key.extract::<String>()?;

            // Convert Python value to SerializableValue
            let scylla_val = py_value_to_serializable(&val)?;

            serialized.insert(key_str, scylla_val);
        }
    }

    Ok(serialized)
}

fn py_value_to_serializable(val: &Bound<'_, PyAny>) -> PyResult<SerializableValue> {
    if val.is_none() {
        return Ok(SerializableValue::Null);
    }

    // Try bool first (before int, as bool is a subclass of int in Python)
    if let Ok(b) = val.extract::<bool>() {
        return Ok(SerializableValue::Bool(b));
    }

    // Try int types
    if let Ok(i) = val.extract::<i32>() {
        return Ok(SerializableValue::Int(i));
    }
    if let Ok(i) = val.extract::<i64>() {
        // Check if this might be a timestamp - timestamps are usually in milliseconds or seconds since epoch
        // Milliseconds: 1000000000000 to 4102444800000 (year 2001 to 2100)
        // Seconds: 1000000000 to 4102444800 (year 2001 to 2100)
        if (1_000_000_000_000..4_102_444_800_000).contains(&i)
            || (1_000_000_000..4_102_444_800).contains(&i)
        {
            // Convert to datetime
            use chrono::TimeZone;
            let (secs, nanos) = if i >= 1_000_000_000_000 {
                // Milliseconds
                (i / 1000, ((i % 1000) * 1_000_000) as u32)
            } else {
                // Seconds
                (i, 0)
            };
            let dt = chrono::Utc
                .timestamp_opt(secs, nanos)
                .single()
                .ok_or_else(|| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timestamp value")
                })?;
            return Ok(SerializableValue::Timestamp(dt));
        }
        return Ok(SerializableValue::BigInt(i));
    }

    // Try float types
    if let Ok(f) = val.extract::<f32>() {
        return Ok(SerializableValue::Float(f));
    }
    if let Ok(f) = val.extract::<f64>() {
        // Check if this is a timestamp (common pattern in Python with datetime.timestamp())
        // If it's a reasonable timestamp value (between 1970 and 2100), treat it as timestamp
        if f > 0.0 && f < 4_102_444_800.0 {
            // Between 1970 and 2100
            // Convert to datetime
            use chrono::TimeZone;
            let dt = chrono::Utc
                .timestamp_opt(f as i64, (f.fract() * 1_000_000_000.0) as u32)
                .single()
                .ok_or_else(|| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timestamp value")
                })?;
            return Ok(SerializableValue::Timestamp(dt));
        }
        return Ok(SerializableValue::Double(f));
    }

    // Try string
    if let Ok(s) = val.extract::<String>() {
        return Ok(SerializableValue::Text(s));
    }

    // Try bytes/blob
    if let Ok(b) = val.extract::<Vec<u8>>() {
        return Ok(SerializableValue::Blob(b));
    }

    // Try list
    if let Ok(list) = val.downcast::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(py_value_to_serializable(&item)?);
        }
        return Ok(SerializableValue::List(items));
    }

    // Try dict (as map)
    if let Ok(dict) = val.downcast::<PyDict>() {
        // Try to detect if it's a text map or int map
        let mut text_map: HashMap<String, String> = HashMap::new();
        let mut int_map: HashMap<String, i64> = HashMap::new();
        let mut is_text_map = true;
        let mut is_int_map = true;

        for (k, v) in dict.iter() {
            let key_str = k.extract::<String>().ok();

            if let Some(key) = &key_str {
                if is_text_map {
                    if let Ok(val_str) = v.extract::<String>() {
                        text_map.insert(key.clone(), val_str);
                    } else {
                        is_text_map = false;
                    }
                }

                if is_int_map {
                    if let Ok(val_int) = v.extract::<i64>() {
                        int_map.insert(key.clone(), val_int);
                    } else {
                        is_int_map = false;
                    }
                }
            } else {
                is_text_map = false;
                is_int_map = false;
            }

            if !is_text_map && !is_int_map {
                break;
            }
        }

        if is_text_map && !text_map.is_empty() {
            return Ok(SerializableValue::TextMap(text_map));
        } else if is_int_map && !int_map.is_empty() {
            return Ok(SerializableValue::IntMap(int_map));
        }

        // If neither worked, return an empty text map as fallback
        return Ok(SerializableValue::TextMap(HashMap::new()));
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
        "Cannot serialize Python type: {:?}",
        val.get_type()
    )))
}
