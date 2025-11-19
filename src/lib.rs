#![allow(unexpected_cfgs)]
#![allow(clippy::useless_conversion)]

use pyo3::prelude::*;

mod batch;
mod error;
mod query;
mod result;
mod session;
mod types;

use batch::Batch;
use error::ScyllaError;
use query::{PreparedStatement, Query};
use result::{QueryResult, Row};
use session::{Session, SessionBuilder};

#[pymodule]
fn _rscylla(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Core types
    m.add_class::<SessionBuilder>()?;
    m.add_class::<Session>()?;
    m.add_class::<Query>()?;
    m.add_class::<PreparedStatement>()?;
    m.add_class::<QueryResult>()?;
    m.add_class::<Row>()?;
    m.add_class::<Batch>()?;

    // Exception
    m.add("ScyllaError", _py.get_type::<ScyllaError>())?;

    Ok(())
}
