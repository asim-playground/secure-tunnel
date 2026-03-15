//! Python extension module for Secure Tunnel.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use secure_tunnel_core as core;

/// Parse and evaluate a simple arithmetic expression.
#[pyfunction]
fn parse(expression: &str) -> PyResult<String> {
    core::parse(expression).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Python module for Secure Tunnel.
#[pymodule]
fn secure_tunnel(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__version__", env!("CARGO_PKG_VERSION"))?;
    m.setattr("__all__", ["__version__", "parse"])?;
    m.setattr(
        "__doc__",
        "Python bindings for Secure Tunnel, a simple arithmetic expression parser.",
    )?;

    m.add_function(wrap_pyfunction!(parse, m)?)?;

    Ok(())
}
