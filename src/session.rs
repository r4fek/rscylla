use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_async_runtimes::tokio::future_into_py;
use scylla::client::session::Session as ScyllaSession;
use scylla::client::session_builder::SessionBuilder as ScyllaSessionBuilder;
use std::sync::Arc;
use std::time::Duration;

use crate::batch::Batch;
use crate::error::{
    prepare_error_to_py, query_error_to_py, schema_agreement_error_to_py, session_error_to_py,
    use_keyspace_error_to_py,
};
use crate::query::{PreparedStatement, Query};
use crate::result::QueryResult;
use crate::types::py_dict_to_serialized_values;

#[pyclass]
#[derive(Clone, Default)]
pub struct SessionBuilder {
    builder: ScyllaSessionBuilder,
}

#[pymethods]
impl SessionBuilder {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn known_node(&mut self, hostname: &str) -> PyResult<Self> {
        self.builder = self.builder.clone().known_node(hostname);
        Ok(self.clone())
    }

    pub fn known_nodes(&mut self, hostnames: Vec<String>) -> PyResult<Self> {
        self.builder = self.builder.clone().known_nodes(&hostnames);
        Ok(self.clone())
    }

    pub fn use_keyspace(&mut self, keyspace_name: &str, case_sensitive: bool) -> PyResult<Self> {
        self.builder = self
            .builder
            .clone()
            .use_keyspace(keyspace_name, case_sensitive);
        Ok(self.clone())
    }

    pub fn connection_timeout(&mut self, duration_ms: u64) -> PyResult<Self> {
        self.builder = self
            .builder
            .clone()
            .connection_timeout(Duration::from_millis(duration_ms));
        Ok(self.clone())
    }

    pub fn pool_size(&mut self, size: usize) -> PyResult<Self> {
        use std::num::NonZeroUsize;
        let non_zero_size = NonZeroUsize::new(size).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Pool size must be greater than 0")
        })?;
        self.builder = self
            .builder
            .clone()
            .pool_size(scylla::client::PoolSize::PerHost(non_zero_size));
        Ok(self.clone())
    }

    pub fn user(&mut self, username: &str, password: &str) -> PyResult<Self> {
        self.builder = self.builder.clone().user(username, password);
        Ok(self.clone())
    }

    #[pyo3(signature = (compression=None))]
    pub fn compression(&mut self, compression: Option<&str>) -> PyResult<Self> {
        let comp = match compression {
            Some("lz4") => Some(scylla::client::Compression::Lz4),
            Some("snappy") => Some(scylla::client::Compression::Snappy),
            None => None,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Invalid compression type. Must be 'lz4', 'snappy', or None",
                ))
            }
        };
        self.builder = self.builder.clone().compression(comp);
        Ok(self.clone())
    }

    pub fn tcp_nodelay(&mut self, nodelay: bool) -> PyResult<Self> {
        self.builder = self.builder.clone().tcp_nodelay(nodelay);
        Ok(self.clone())
    }

    #[pyo3(signature = (_keepalive_ms=None))]
    pub fn tcp_keepalive(&mut self, _keepalive_ms: Option<u64>) -> PyResult<Self> {
        // Note: tcp_keepalive method has been removed from the scylla crate
        // TCP keepalive configuration should be done at the OS level
        Ok(self.clone())
    }

    pub fn build<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let builder = self.builder.clone();

        future_into_py(py, async move {
            let session = builder.build().await.map_err(session_error_to_py)?;

            Ok(Session {
                session: Arc::new(session),
            })
        })
    }
}

#[pyclass]
#[derive(Clone)]
pub struct Session {
    pub(crate) session: Arc<ScyllaSession>,
}

#[pymethods]
impl Session {
    #[staticmethod]
    pub fn connect<'py>(py: Python<'py>, nodes: Vec<String>) -> PyResult<Bound<'py, PyAny>> {
        let mut builder = SessionBuilder::new();
        builder.known_nodes(nodes)?;
        builder.build(py)
    }

    #[pyo3(signature = (query, values=None))]
    pub fn execute<'py>(
        &self,
        py: Python<'py>,
        query: &str,
        values: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let serialized_values = py_dict_to_serialized_values(values)?;

        let session = self.session.clone();
        let query_str = query.to_string();

        future_into_py(py, async move {
            let result = session
                .query_unpaged(query_str, serialized_values)
                .await
                .map_err(query_error_to_py)?;

            Ok(QueryResult::new(result))
        })
    }

    #[pyo3(signature = (query, values=None))]
    pub fn query<'py>(
        &self,
        py: Python<'py>,
        query: &Query,
        values: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let serialized_values = py_dict_to_serialized_values(values)?;

        let session = self.session.clone();
        let scylla_query = query.inner.clone();

        future_into_py(py, async move {
            let result = session
                .query_unpaged(scylla_query, serialized_values)
                .await
                .map_err(query_error_to_py)?;

            Ok(QueryResult::new(result))
        })
    }

    pub fn prepare<'py>(&self, py: Python<'py>, query: &str) -> PyResult<Bound<'py, PyAny>> {
        let session = self.session.clone();
        let query_str = query.to_string();

        future_into_py(py, async move {
            let prepared = session
                .prepare(query_str)
                .await
                .map_err(prepare_error_to_py)?;

            Ok(PreparedStatement {
                prepared: Arc::new(prepared),
            })
        })
    }

    #[pyo3(signature = (prepared, values=None))]
    pub fn execute_prepared<'py>(
        &self,
        py: Python<'py>,
        prepared: &PreparedStatement,
        values: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let serialized_values = py_dict_to_serialized_values(values)?;

        let session = self.session.clone();
        let prep = prepared.prepared.clone();

        future_into_py(py, async move {
            let result = session
                .execute_unpaged(&prep, serialized_values)
                .await
                .map_err(query_error_to_py)?;

            Ok(QueryResult::new(result))
        })
    }

    pub fn batch<'py>(
        &self,
        py: Python<'py>,
        batch: &Batch,
        values: &Bound<'_, PyList>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let session = self.session.clone();
        let scylla_batch = batch.inner.clone();

        let mut batch_values = Vec::new();
        for item in values.iter() {
            if let Ok(dict) = item.cast::<PyDict>() {
                let serialized = py_dict_to_serialized_values(Some(dict))?;
                batch_values.push(serialized);
            } else {
                batch_values.push(py_dict_to_serialized_values(None)?);
            }
        }

        future_into_py(py, async move {
            let result = session
                .batch(&scylla_batch, batch_values)
                .await
                .map_err(query_error_to_py)?;

            Ok(QueryResult::new(result))
        })
    }

    pub fn use_keyspace<'py>(
        &self,
        py: Python<'py>,
        keyspace_name: &str,
        case_sensitive: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let session = self.session.clone();
        let ks = keyspace_name.to_string();

        future_into_py(py, async move {
            session
                .use_keyspace(ks, case_sensitive)
                .await
                .map_err(use_keyspace_error_to_py)?;

            Ok(())
        })
    }

    pub fn await_schema_agreement<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let session = self.session.clone();

        future_into_py(py, async move {
            session
                .await_schema_agreement()
                .await
                .map_err(schema_agreement_error_to_py)?;

            // Return True when schema agreement is reached
            Ok(true)
        })
    }

    pub fn get_cluster_data(&self) -> PyResult<String> {
        // ClusterData doesn't implement Debug, so we return a simple message
        Ok("ClusterData available (not serializable)".to_string())
    }

    pub fn get_keyspace(&self) -> Option<String> {
        self.session.get_keyspace().map(|s| s.to_string())
    }
}
