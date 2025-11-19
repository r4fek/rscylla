use pyo3::prelude::*;
use scylla::statement::batch::Batch as ScyllaBatch;
use scylla::statement::Consistency;

use crate::query::{PreparedStatement, Query};

#[pyclass]
#[derive(Clone)]
pub struct Batch {
    pub(crate) inner: ScyllaBatch,
}

#[pymethods]
impl Batch {
    #[new]
    #[pyo3(signature = (batch_type="logged"))]
    pub fn new(batch_type: &str) -> PyResult<Self> {
        let btype = match batch_type.to_lowercase().as_str() {
            "logged" => scylla::statement::batch::BatchType::Logged,
            "unlogged" => scylla::statement::batch::BatchType::Unlogged,
            "counter" => scylla::statement::batch::BatchType::Counter,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Invalid batch type. Must be 'logged', 'unlogged', or 'counter'",
                ))
            }
        };

        Ok(Batch {
            inner: ScyllaBatch::new(btype),
        })
    }

    pub fn append_statement(&mut self, query: &str) {
        self.inner.append_statement(query);
    }

    pub fn append_query(&mut self, query: &Query) {
        self.inner.append_statement(query.inner.clone());
    }

    pub fn append_prepared(&mut self, prepared: &PreparedStatement) {
        self.inner.append_statement((*prepared.prepared).clone());
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

    pub fn with_timestamp(&mut self, timestamp: i64) -> PyResult<Self> {
        self.inner.set_timestamp(Some(timestamp));
        Ok(self.clone())
    }

    pub fn with_timeout(&mut self, _timeout_ms: u64) -> PyResult<Self> {
        // Note: set_request_timeout has been removed from the scylla crate
        // Timeout configuration should be done at the session level
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

    pub fn statements_count(&self) -> usize {
        self.inner.statements.len()
    }

    pub fn __repr__(&self) -> String {
        format!("Batch(statements={})", self.inner.statements.len())
    }
}

fn parse_consistency(consistency: &str) -> PyResult<Consistency> {
    match consistency.to_uppercase().as_str() {
        "ANY" => Ok(Consistency::Any),
        "ONE" => Ok(Consistency::One),
        "TWO" => Ok(Consistency::Two),
        "THREE" => Ok(Consistency::Three),
        "QUORUM" => Ok(Consistency::Quorum),
        "ALL" => Ok(Consistency::All),
        "LOCAL_QUORUM" | "LOCALQUORUM" => Ok(Consistency::LocalQuorum),
        "EACH_QUORUM" | "EACHQUORUM" => Ok(Consistency::EachQuorum),
        "LOCAL_ONE" | "LOCALONE" => Ok(Consistency::LocalOne),
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
