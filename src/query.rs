use pyo3::prelude::*;
use pyo3::types::PyBytes;
use scylla::statement::prepared::PreparedStatement as ScyllaPreparedStatement;
use scylla::statement::unprepared::Statement as ScyllaQuery;
use std::sync::Arc;
use std::time::Duration;

#[pyclass]
#[derive(Clone)]
pub struct Query {
    pub(crate) inner: ScyllaQuery,
    query_string: String,
}

#[pymethods]
impl Query {
    #[new]
    pub fn new(query: &str) -> Self {
        Query {
            inner: ScyllaQuery::new(query),
            query_string: query.to_string(),
        }
    }

    pub fn with_consistency(&mut self, consistency: &str) -> PyResult<Self> {
        let cons = parse_consistency(consistency)?;
        self.inner.set_consistency(cons);
        Ok(self.clone())
    }

    pub fn with_serial_consistency(&mut self, serial_consistency: &str) -> PyResult<Self> {
        let cons = parse_serial_consistency(serial_consistency)?;
        self.inner.set_serial_consistency(Some(cons));
        Ok(self.clone())
    }

    pub fn with_page_size(&mut self, page_size: i32) -> PyResult<Self> {
        self.inner.set_page_size(page_size);
        Ok(self.clone())
    }

    pub fn with_timestamp(&mut self, timestamp: i64) -> PyResult<Self> {
        self.inner.set_timestamp(Some(timestamp));
        Ok(self.clone())
    }

    pub fn with_timeout(&mut self, timeout_ms: u64) -> PyResult<Self> {
        self.inner
            .set_request_timeout(Some(Duration::from_millis(timeout_ms)));
        Ok(self.clone())
    }

    pub fn with_tracing(&mut self, tracing: bool) -> PyResult<Self> {
        self.inner.set_tracing(tracing);
        Ok(self.clone())
    }

    pub fn is_idempotent(&self) -> bool {
        self.inner.get_is_idempotent()
    }

    pub fn set_idempotent(&mut self, idempotent: bool) {
        self.inner.set_is_idempotent(idempotent);
    }

    pub fn get_contents(&self) -> String {
        self.query_string.clone()
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PreparedStatement {
    pub(crate) prepared: Arc<ScyllaPreparedStatement>,
}

#[pymethods]
impl PreparedStatement {
    pub fn with_consistency(&self, consistency: &str) -> PyResult<Self> {
        let cons = parse_consistency(consistency)?;
        let mut new_prepared = (*self.prepared).clone();
        new_prepared.set_consistency(cons);
        Ok(PreparedStatement {
            prepared: Arc::new(new_prepared),
        })
    }

    pub fn with_serial_consistency(&self, serial_consistency: &str) -> PyResult<Self> {
        let cons = parse_serial_consistency(serial_consistency)?;
        let mut new_prepared = (*self.prepared).clone();
        new_prepared.set_serial_consistency(Some(cons));
        Ok(PreparedStatement {
            prepared: Arc::new(new_prepared),
        })
    }

    pub fn with_page_size(&self, page_size: i32) -> PyResult<Self> {
        let mut new_prepared = (*self.prepared).clone();
        new_prepared.set_page_size(page_size);
        Ok(PreparedStatement {
            prepared: Arc::new(new_prepared),
        })
    }

    pub fn with_timestamp(&self, timestamp: i64) -> PyResult<Self> {
        let mut new_prepared = (*self.prepared).clone();
        new_prepared.set_timestamp(Some(timestamp));
        Ok(PreparedStatement {
            prepared: Arc::new(new_prepared),
        })
    }

    pub fn with_tracing(&self, tracing: bool) -> PyResult<Self> {
        let mut new_prepared = (*self.prepared).clone();
        new_prepared.set_tracing(tracing);
        Ok(PreparedStatement {
            prepared: Arc::new(new_prepared),
        })
    }

    pub fn is_idempotent(&self) -> bool {
        self.prepared.get_is_idempotent()
    }

    pub fn set_idempotent(&self, idempotent: bool) -> Self {
        let mut new_prepared = (*self.prepared).clone();
        new_prepared.set_is_idempotent(idempotent);
        PreparedStatement {
            prepared: Arc::new(new_prepared),
        }
    }

    pub fn get_id<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, self.prepared.get_id()))
    }

    pub fn get_statement(&self) -> String {
        self.prepared.get_statement().to_string()
    }
}

fn parse_consistency(consistency: &str) -> PyResult<scylla::statement::Consistency> {
    match consistency.to_uppercase().as_str() {
        "ANY" => Ok(scylla::statement::Consistency::Any),
        "ONE" => Ok(scylla::statement::Consistency::One),
        "TWO" => Ok(scylla::statement::Consistency::Two),
        "THREE" => Ok(scylla::statement::Consistency::Three),
        "QUORUM" => Ok(scylla::statement::Consistency::Quorum),
        "ALL" => Ok(scylla::statement::Consistency::All),
        "LOCAL_QUORUM" | "LOCALQUORUM" => Ok(scylla::statement::Consistency::LocalQuorum),
        "EACH_QUORUM" | "EACHQUORUM" => Ok(scylla::statement::Consistency::EachQuorum),
        "LOCAL_ONE" | "LOCALONE" => Ok(scylla::statement::Consistency::LocalOne),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid consistency level: {}",
            consistency
        ))),
    }
}

fn parse_serial_consistency(consistency: &str) -> PyResult<scylla::statement::SerialConsistency> {
    match consistency.to_uppercase().as_str() {
        "SERIAL" => Ok(scylla::statement::SerialConsistency::Serial),
        "LOCAL_SERIAL" | "LOCALSERIAL" => Ok(scylla::statement::SerialConsistency::LocalSerial),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid serial consistency level: {}",
            consistency
        ))),
    }
}
