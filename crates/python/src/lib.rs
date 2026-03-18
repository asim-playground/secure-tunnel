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

/// Return the stable v1 protocol identifier.
#[pyfunction]
#[allow(clippy::missing_const_for_fn)]
fn protocol_id_v1() -> &'static str {
    core::PROTOCOL_ID_V1
}

/// Return the v1 `QUIC` ALPN value.
#[pyfunction]
#[allow(clippy::missing_const_for_fn)]
fn quic_alpn_v1() -> &'static str {
    core::QUIC_ALPN_V1
}

/// Return the v1 `WSS` subprotocol value.
#[pyfunction]
#[allow(clippy::missing_const_for_fn)]
fn wss_subprotocol_v1() -> &'static str {
    core::WSS_SUBPROTOCOL_V1
}

/// Return a sample service descriptor as JSON.
#[pyfunction]
fn example_service_descriptor_json() -> PyResult<String> {
    serde_json::to_string(&core::example_service_descriptor())
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// Validate a service descriptor JSON string.
#[pyfunction]
fn validate_service_descriptor_json(descriptor_json: &str) -> PyResult<()> {
    let descriptor = decode_descriptor(descriptor_json)?;
    descriptor
        .validate()
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// Decode, validate, and re-encode a service descriptor JSON string.
#[pyfunction]
fn normalize_service_descriptor_json(descriptor_json: &str) -> PyResult<String> {
    let descriptor = decode_descriptor(descriptor_json)?;
    descriptor
        .validate()
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    serde_json::to_string(&descriptor).map_err(|error| PyValueError::new_err(error.to_string()))
}

/// Python module for Secure Tunnel.
#[pymodule]
fn secure_tunnel(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__version__", env!("CARGO_PKG_VERSION"))?;
    m.setattr(
        "__all__",
        [
            "__version__",
            "protocol_id_v1",
            "quic_alpn_v1",
            "wss_subprotocol_v1",
            "example_service_descriptor_json",
            "validate_service_descriptor_json",
            "normalize_service_descriptor_json",
        ],
    )?;
    m.setattr(
        "__doc__",
        "Python bindings for Secure Tunnel protocol metadata and descriptor validation.",
    )?;

    m.add_function(wrap_pyfunction!(protocol_id_v1, m)?)?;
    m.add_function(wrap_pyfunction!(quic_alpn_v1, m)?)?;
    m.add_function(wrap_pyfunction!(wss_subprotocol_v1, m)?)?;
    m.add_function(wrap_pyfunction!(example_service_descriptor_json, m)?)?;
    m.add_function(wrap_pyfunction!(validate_service_descriptor_json, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_service_descriptor_json, m)?)?;

    Ok(())
}

fn decode_descriptor(descriptor_json: &str) -> PyResult<core::ServiceDescriptor> {
    serde_json::from_str(descriptor_json).map_err(|error| PyValueError::new_err(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        example_service_descriptor_json, normalize_service_descriptor_json, protocol_id_v1,
        secure_tunnel, validate_service_descriptor_json,
    };
    use pyo3::prelude::*;
    use pyo3::types::PyModule;

    #[test]
    fn protocol_metadata_matches_core_constants() {
        assert_eq!(protocol_id_v1(), secure_tunnel_core::PROTOCOL_ID_V1);
    }

    #[test]
    fn example_descriptor_validates() {
        let descriptor = example_service_descriptor_json().expect("example descriptor encodes");
        validate_service_descriptor_json(&descriptor).expect("example descriptor validates");
    }

    #[test]
    fn invalid_descriptor_maps_to_value_error() {
        let descriptor = example_service_descriptor_json()
            .expect("example descriptor encodes")
            .replace(
                "\"protocol_id\":\"secure-tunnel-v1\"",
                "\"protocol_id\":\"wrong\"",
            );

        let err = normalize_service_descriptor_json(&descriptor).expect_err("validation fails");
        Python::initialize();
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

            let protocol_fn = module
                .getattr("protocol_id_v1")
                .expect("protocol function should be exported");
            let value = protocol_fn
                .call0()
                .expect("protocol function should be callable")
                .extract::<String>()
                .expect("protocol id should be a string");
            assert_eq!(value, secure_tunnel_core::PROTOCOL_ID_V1);
        });
    }
}
