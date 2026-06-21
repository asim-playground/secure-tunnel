// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::future::ready;

pub(super) trait TransportPorts: Send + Sync {
    fn quic(&self) -> Option<&dyn secure_tunnel_core::CarrierConnector>;
    fn wss(&self) -> Option<&dyn secure_tunnel_core::CarrierConnector>;
    fn secure_ready(&self) -> &dyn secure_tunnel_core::SecureReadyEvaluator;
}

#[derive(Default)]
pub(super) struct UnavailableTransportPorts {
    secure_ready: UnavailableSecureReady,
}

impl TransportPorts for UnavailableTransportPorts {
    fn quic(&self) -> Option<&dyn secure_tunnel_core::CarrierConnector> {
        None
    }

    fn wss(&self) -> Option<&dyn secure_tunnel_core::CarrierConnector> {
        None
    }

    fn secure_ready(&self) -> &dyn secure_tunnel_core::SecureReadyEvaluator {
        &self.secure_ready
    }
}

#[derive(Default)]
struct UnavailableSecureReady;

impl secure_tunnel_core::SecureReadyEvaluator for UnavailableSecureReady {
    fn reach_secure_ready(
        &self,
        _descriptor: &secure_tunnel_core::ServiceDescriptor,
        _now_unix_seconds: u64,
        _transport: Box<dyn secure_tunnel_core::FramedDuplex>,
    ) -> secure_tunnel_core::BoxFuture<
        '_,
        secure_tunnel_core::ApiResult<secure_tunnel_core::SecureReadyTransport>,
    > {
        Box::pin(ready(Err(
            secure_tunnel_core::ApiError::TransportSelectorInvariant(
                "secure-ready evaluator is not configured",
            ),
        )))
    }
}
