// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

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

#[cfg(test)]
mod tests {
    use super::{parse, secure_tunnel};
    use pyo3::prelude::*;
    use pyo3::types::PyModule;

    #[test]
    fn parse_returns_sum() {
        let result = parse("1 + 2 + 3").expect("parse should succeed");
        assert_eq!(result, "6");
    }

    #[test]
    fn parse_maps_core_errors() {
        Python::initialize();
        let err = parse("1 +").expect_err("parse should fail");
        Python::attach(|py| {
            assert!(err.is_instance_of::<pyo3::exceptions::PyValueError>(py));
        });
    }

    #[test]
    fn module_exports_expected_api() {
        Python::initialize();
        Python::attach(|py| {
            let module = PyModule::new(py, "secure_tunnel").expect("module should be created");
            secure_tunnel(py, &module).expect("module initialization should succeed");

            let version = module
                .getattr("__version__")
                .expect("version should be set")
                .extract::<String>()
                .expect("version should be a string");
            assert_eq!(version, env!("CARGO_PKG_VERSION"));

            let parse_fn = module
                .getattr("parse")
                .expect("parse function should be exported");
            let value = parse_fn
                .call1(("10+20+30",))
                .expect("parse function should be callable")
                .extract::<String>()
                .expect("parse result should be a string");
            assert_eq!(value, "60");
        });
    }
}
