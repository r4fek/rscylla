use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use scylla::response::query_result::{QueryResult as ScyllaQueryResult, QueryRowsResult};
use scylla::value::{CqlValue, Row as ScyllaRow};

use crate::types::cql_value_to_py;

#[pyclass]
pub struct QueryResult {
    // Store the rows result if available
    rows_result: Option<QueryRowsResult>,
    tracing_id: Option<String>,
    warnings: Vec<String>,
    current_row: usize,
}

impl QueryResult {
    pub fn new(result: ScyllaQueryResult) -> Self {
        let tracing_id = result.tracing_id().map(|id| id.to_string());
        let warnings: Vec<String> = result.warnings().map(|s| s.to_string()).collect();
        let rows_result = result.into_rows_result().ok();

        QueryResult {
            rows_result,
            tracing_id,
            warnings,
            current_row: 0,
        }
    }
}

#[pymethods]
impl QueryResult {
    pub fn rows(&self, py: Python) -> PyResult<Py<PyAny>> {
        let py_list = PyList::empty(py);

        if let Some(ref rows_result) = self.rows_result {
            let rows: Vec<ScyllaRow> = rows_result
                .rows()
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Row deserialization error: {}",
                        e
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Row deserialization error: {}",
                        e
                    ))
                })?;

            for row in rows {
                let py_row = Py::new(py, Row::new(&row))?;
                py_list.append(py_row)?;
            }
        }

        Ok(py_list.into())
    }

    pub fn first_row(&self) -> PyResult<Option<Row>> {
        if let Some(ref rows_result) = self.rows_result {
            let mut rows_iter = rows_result.rows().map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Row deserialization error: {}",
                    e
                ))
            })?;

            if let Some(row_result) = rows_iter.next() {
                let row: ScyllaRow = row_result.map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Row deserialization error: {}",
                        e
                    ))
                })?;
                Ok(Some(Row::new(&row)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn single_row(&self) -> PyResult<Row> {
        if let Some(ref rows_result) = self.rows_result {
            let rows: Vec<ScyllaRow> = rows_result
                .rows()
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Row deserialization error: {}",
                        e
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Row deserialization error: {}",
                        e
                    ))
                })?;

            if rows.len() == 1 {
                Ok(Row::new(&rows[0]))
            } else if rows.is_empty() {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "No rows returned",
                ))
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

    pub fn first_row_typed(&self, py: Python) -> PyResult<Option<Py<PyAny>>> {
        if let Some(ref rows_result) = self.rows_result {
            let mut rows_iter = rows_result.rows().map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Row deserialization error: {}",
                    e
                ))
            })?;

            if let Some(row_result) = rows_iter.next() {
                let row: ScyllaRow = row_result.map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Row deserialization error: {}",
                        e
                    ))
                })?;
                let py_row = Row::new(&row);
                Ok(Some(py_row.as_dict(py)?))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn rows_typed(&self, py: Python) -> PyResult<Vec<Py<PyAny>>> {
        let mut result = Vec::new();

        if let Some(ref rows_result) = self.rows_result {
            let rows: Vec<ScyllaRow> = rows_result
                .rows()
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Row deserialization error: {}",
                        e
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Row deserialization error: {}",
                        e
                    ))
                })?;

            for row in rows {
                let py_row = Row::new(&row);
                result.push(py_row.as_dict(py)?);
            }
        }

        Ok(result)
    }

    pub fn col_specs(&self, py: Python) -> PyResult<Py<PyAny>> {
        let py_list = PyList::empty(py);

        if let Some(ref rows_result) = self.rows_result {
            let specs = rows_result.column_specs();
            for spec in specs.iter() {
                let dict = PyDict::new(py);
                dict.set_item("table_spec", format!("{:?}", spec.table_spec()))?;
                dict.set_item("name", spec.name().to_string())?;
                dict.set_item("typ", format!("{:?}", spec.typ()))?;
                py_list.append(dict)?;
            }
        }

        Ok(py_list.into())
    }

    pub fn tracing_id(&self) -> Option<String> {
        self.tracing_id.clone()
    }

    pub fn warnings(&self) -> Vec<String> {
        self.warnings.clone()
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Row> {
        if let Some(ref rows_result) = slf.rows_result {
            if let Ok(rows) = rows_result.rows::<ScyllaRow>() {
                let rows_vec: Vec<ScyllaRow> = rows.filter_map(|r| r.ok()).collect();
                if slf.current_row < rows_vec.len() {
                    let row = Row::new(&rows_vec[slf.current_row]);
                    slf.current_row += 1;
                    return Some(row);
                }
            }
        }
        None
    }

    pub fn __len__(&self) -> usize {
        if let Some(ref rows_result) = self.rows_result {
            rows_result.rows_num()
        } else {
            0
        }
    }

    pub fn __bool__(&self) -> bool {
        if let Some(ref rows_result) = self.rows_result {
            rows_result.rows_num() > 0
        } else {
            false
        }
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
    pub fn columns(&self, py: Python) -> PyResult<Py<PyAny>> {
        let py_list = PyList::empty(py);
        for column in &self.columns {
            let value = match column {
                Some(val) => cql_value_to_py(py, val)?,
                None => py.None(),
            };
            py_list.append(value)?;
        }
        Ok(py_list.into())
    }

    pub fn as_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);

        // Note: In a real implementation, you'd need column names from the result metadata
        // For now, we'll use indices as keys
        for (i, column) in self.columns.iter().enumerate() {
            let value = match column {
                Some(val) => cql_value_to_py(py, val)?,
                None => py.None(),
            };
            dict.set_item(format!("col_{}", i), value)?;
        }

        Ok(dict.into())
    }

    pub fn get(&self, py: Python, index: usize) -> PyResult<Py<PyAny>> {
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

    pub fn __getitem__(&self, py: Python, index: isize) -> PyResult<Py<PyAny>> {
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
