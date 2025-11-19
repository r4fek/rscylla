use pyo3::create_exception;
use pyo3::prelude::*;

create_exception!(rscylla, ScyllaError, pyo3::exceptions::PyException);

// Helper functions to convert scylla errors to PyErr
// We can't implement From directly due to orphan rules
pub fn query_error_to_py(err: scylla::errors::ExecutionError) -> PyErr {
    PyErr::new::<ScyllaError, _>(format!("Query error: {}", err))
}

pub fn session_error_to_py(err: scylla::errors::NewSessionError) -> PyErr {
    PyErr::new::<ScyllaError, _>(format!("Session error: {}", err))
}

pub fn prepare_error_to_py(err: scylla::errors::PrepareError) -> PyErr {
    PyErr::new::<ScyllaError, _>(format!("Prepare error: {}", err))
}

pub fn use_keyspace_error_to_py(err: scylla::errors::UseKeyspaceError) -> PyErr {
    PyErr::new::<ScyllaError, _>(format!("Use keyspace error: {}", err))
}

pub fn schema_agreement_error_to_py(err: scylla::errors::SchemaAgreementError) -> PyErr {
    PyErr::new::<ScyllaError, _>(format!("Schema agreement error: {}", err))
}

#[allow(dead_code)]
pub fn serialization_error_to_py(err: scylla::serialize::SerializationError) -> PyErr {
    PyErr::new::<ScyllaError, _>(format!("Serialization error: {}", err))
}

#[allow(dead_code)]
pub fn deserialization_error_to_py(err: scylla::deserialize::DeserializationError) -> PyErr {
    PyErr::new::<ScyllaError, _>(format!("Deserialization error: {}", err))
}
