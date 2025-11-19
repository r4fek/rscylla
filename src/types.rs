use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use scylla::frame::response::result::CqlValue;
use scylla::frame::value::LegacySerializedValues;
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

pub fn py_dict_to_serialized_values(
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<LegacySerializedValues> {
    let mut serialized = LegacySerializedValues::new();

    if let Some(d) = dict {
        // Extract all keys and values while preserving Python dict order
        // Use keys() and get_item() to preserve order since PyDict.iter() doesn't guarantee order
        let keys = d.call_method0("keys")?;

        for key_obj_result in keys.iter()? {
            let key_obj = key_obj_result?;
            let key_str = key_obj.extract::<String>()?;
            let val = d.get_item(&key_str)?.ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                    "Key '{}' not found",
                    key_str
                ))
            })?;

            // Serialize based on Python type
            if val.is_none() {
                serialized
                    .add_named_value(&key_str, &Option::<i32>::None)
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Serialization error: {}",
                            e
                        ))
                    })?;
            } else if let Ok(b) = val.extract::<bool>() {
                serialized.add_named_value(&key_str, &b).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Serialization error: {}",
                        e
                    ))
                })?;
            } else if let Ok(i) = val.extract::<i64>() {
                // Use i32 for small integers, i64 for large ones
                // This works for most cases; bigint/counter columns may need explicit i64 via prepared statements
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    serialized
                        .add_named_value(&key_str, &(i as i32))
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                "Serialization error: {}",
                                e
                            ))
                        })?;
                } else {
                    serialized.add_named_value(&key_str, &i).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Serialization error: {}",
                            e
                        ))
                    })?;
                }
            } else if let Ok(f) = val.extract::<f64>() {
                // Use f32 for normal floats, f64 for doubles
                let f32_val = f as f32;
                if f.is_finite() && f.abs() <= f32::MAX as f64 && (f32_val as f64 - f).abs() < 1e-6
                {
                    serialized
                        .add_named_value(&key_str, &f32_val)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                "Serialization error: {}",
                                e
                            ))
                        })?;
                } else {
                    serialized.add_named_value(&key_str, &f).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Serialization error: {}",
                            e
                        ))
                    })?;
                }
            } else if let Ok(s) = val.extract::<String>() {
                serialized.add_named_value(&key_str, &s).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Serialization error: {}",
                        e
                    ))
                })?;
            } else if let Ok(b) = val.extract::<Vec<u8>>() {
                serialized.add_named_value(&key_str, &b).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Serialization error: {}",
                        e
                    ))
                })?;
            } else if let Ok(dict) = val.downcast::<PyDict>() {
                // Handle nested dict (map type) - try String->String first
                let mut string_map: HashMap<String, String> = HashMap::new();
                let mut all_strings = true;

                for (k, v) in dict.iter() {
                    if let (Ok(map_key), Ok(map_value)) =
                        (k.extract::<String>(), v.extract::<String>())
                    {
                        string_map.insert(map_key, map_value);
                    } else {
                        all_strings = false;
                        break;
                    }
                }

                if all_strings && !string_map.is_empty() {
                    serialized
                        .add_named_value(&key_str, &string_map)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                "Serialization error: {}",
                                e
                            ))
                        })?;
                } else {
                    // Try i64 map
                    let mut int_map: HashMap<String, i64> = HashMap::new();
                    for (k, v) in dict.iter() {
                        if let (Ok(map_key), Ok(map_value)) =
                            (k.extract::<String>(), v.extract::<i64>())
                        {
                            int_map.insert(map_key, map_value);
                        }
                    }
                    serialized
                        .add_named_value(&key_str, &int_map)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                                "Serialization error: {}",
                                e
                            ))
                        })?;
                }
            } else if let Ok(list) = val.downcast::<PyList>() {
                let mut vec_values: Vec<String> = Vec::new();
                for list_item in list.iter() {
                    if let Ok(s) = list_item.extract::<String>() {
                        vec_values.push(s);
                    }
                }
                serialized
                    .add_named_value(&key_str, &vec_values)
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Serialization error: {}",
                            e
                        ))
                    })?;
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                    "Cannot serialize Python type for key '{}': {:?}",
                    key_str,
                    val.get_type()
                )));
            }
        }
    }

    Ok(serialized)
}
