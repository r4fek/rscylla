use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use scylla::frame::response::result::{CqlValue, Row as ScyllaRow};
use scylla::QueryResult as ScyllaQueryResult;

use crate::types::cql_value_to_py;

#[pyclass]
pub struct QueryResult {
    result: ScyllaQueryResult,
    current_row: usize,
}

impl QueryResult {
    pub fn new(result: ScyllaQueryResult) -> Self {
        QueryResult {
            result,
            current_row: 0,
        }
    }
}

#[pymethods]
impl QueryResult {
    pub fn rows(&self, py: Python) -> PyResult<PyObject> {
        if let Some(rows) = &self.result.rows {
            let py_list = PyList::empty_bound(py);
            for row in rows {
                let py_row = Py::new(py, Row::new(row))?;
                py_list.append(py_row)?;
            }
            Ok(py_list.to_object(py))
        } else {
            Ok(PyList::empty_bound(py).to_object(py))
        }
    }

    pub fn first_row(&self) -> PyResult<Option<Row>> {
        if let Some(rows) = &self.result.rows {
            Ok(rows.first().map(Row::new))
        } else {
            Ok(None)
        }
    }

    pub fn single_row(&self) -> PyResult<Row> {
        if let Some(rows) = &self.result.rows {
            if rows.len() == 1 {
                Ok(Row::new(&rows[0]))
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Expected single row, got {} rows",
                    rows.len()
                )))
            }
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "No rows returned",
            ))
        }
    }

    pub fn first_row_typed(&self, py: Python) -> PyResult<Option<PyObject>> {
        if let Some(rows) = &self.result.rows {
            if let Some(row) = rows.first() {
                let py_row = Row::new(row);
                Ok(Some(py_row.as_dict(py)?))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn rows_typed(&self, py: Python) -> PyResult<Vec<PyObject>> {
        if let Some(rows) = &self.result.rows {
            let mut result = Vec::new();
            for row in rows {
                let py_row = Row::new(row);
                result.push(py_row.as_dict(py)?);
            }
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn col_specs(&self, py: Python) -> PyResult<PyObject> {
        let specs = self.result.col_specs();
        let py_list = PyList::empty_bound(py);

        for spec in specs {
            let dict = PyDict::new_bound(py);
            dict.set_item("table_spec", format!("{:?}", spec.table_spec))?;
            dict.set_item("name", spec.name.clone())?;
            dict.set_item("typ", format!("{:?}", spec.typ))?;
            py_list.append(dict)?;
        }

        Ok(py_list.to_object(py))
    }

    pub fn tracing_id(&self) -> Option<String> {
        self.result.tracing_id.map(|id| id.to_string())
    }

    pub fn warnings(&self) -> Vec<String> {
        self.result.warnings.clone()
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Row> {
        if let Some(rows) = &slf.result.rows {
            if slf.current_row < rows.len() {
                let row = Row::new(&rows[slf.current_row]);
                slf.current_row += 1;
                Some(row)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn __len__(&self) -> usize {
        self.result.rows.as_ref().map(|r| r.len()).unwrap_or(0)
    }

    pub fn __bool__(&self) -> bool {
        self.result.rows.is_some() && !self.result.rows.as_ref().unwrap().is_empty()
    }
}

#[pyclass]
#[derive(Clone)]
pub struct Row {
    columns: Vec<Option<CqlValue>>,
}

impl Row {
    pub fn new(row: &ScyllaRow) -> Self {
        Row {
            columns: row.columns.clone(),
        }
    }
}

#[pymethods]
impl Row {
    pub fn columns(&self, py: Python) -> PyResult<PyObject> {
        let py_list = PyList::empty_bound(py);
        for column in &self.columns {
            let value = match column {
                Some(val) => cql_value_to_py(py, val)?,
                None => py.None(),
            };
            py_list.append(value)?;
        }
        Ok(py_list.to_object(py))
    }

    pub fn as_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);

        // Note: In a real implementation, you'd need column names from the result metadata
        // For now, we'll use indices as keys
        for (i, column) in self.columns.iter().enumerate() {
            let value = match column {
                Some(val) => cql_value_to_py(py, val)?,
                None => py.None(),
            };
            dict.set_item(format!("col_{}", i), value)?;
        }

        Ok(dict.to_object(py))
    }

    pub fn get(&self, py: Python, index: usize) -> PyResult<PyObject> {
        if index < self.columns.len() {
            match &self.columns[index] {
                Some(val) => cql_value_to_py(py, val),
                None => Ok(py.None()),
            }
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(format!(
                "Column index {} out of range",
                index
            )))
        }
    }

    pub fn __len__(&self) -> usize {
        self.columns.len()
    }

    pub fn __getitem__(&self, py: Python, index: isize) -> PyResult<PyObject> {
        let len = self.columns.len() as isize;
        let idx = if index < 0 {
            (len + index) as usize
        } else {
            index as usize
        };

        if idx < self.columns.len() {
            match &self.columns[idx] {
                Some(val) => cql_value_to_py(py, val),
                None => Ok(py.None()),
            }
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(format!(
                "Column index {} out of range",
                index
            )))
        }
    }

    pub fn __repr__(&self) -> String {
        format!("Row(columns={})", self.columns.len())
    }
}
