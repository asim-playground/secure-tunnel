// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use secure_tunnel_core::{
    ApiError, ApiResult, CarrierKind, NOISE_SUITE_V1, QUIC_ALPN_V1, ServiceDescriptor,
    WSS_SUBPROTOCOL_V1, example_service_descriptor,
};

use crate::responder::NoiseServiceResponder;

pub const SMOKE_PING: &[u8] = b"smoke-ping";
pub const SMOKE_PONG: &[u8] = b"smoke-pong";

#[derive(Clone)]
pub struct LocalServiceFixture {
    descriptor: Arc<Mutex<ServiceDescriptor>>,
    server_private_key: Vec<u8>,
    server_public_key: [u8; 32],
    device_public_key: [u8; 32],
}

impl LocalServiceFixture {
    pub fn new(device_public_key: [u8; 32]) -> ApiResult<Self> {
        let mut descriptor = example_service_descriptor();
        let keypair = snow::Builder::new(noise_params()?)
            .generate_keypair()
            .map_err(|_| ApiError::InnerNoiseFailure)?;
        let server_public_key: [u8; 32] = keypair
            .public
            .as_slice()
            .try_into()
            .map_err(|_| ApiError::InnerNoiseFailure)?;
        descriptor.service_static_public_key = STANDARD.encode(server_public_key);
        descriptor.resign_with_example_key_for_testing()?;
        Ok(Self {
            descriptor: Arc::new(Mutex::new(descriptor)),
            server_private_key: keypair.private,
            server_public_key,
            device_public_key,
        })
    }

    pub fn descriptor_for_ports(
        &self,
        quic_port: u16,
        wss_port: u16,
    ) -> ApiResult<ServiceDescriptor> {
        let mut descriptor = self.lock_descriptor()?;
        let quic = descriptor
            .carriers
            .quic
            .as_mut()
            .ok_or(ApiError::UnavailableCarrier(CarrierKind::Quic))?;
        "127.0.0.1".clone_into(&mut quic.connect_host);
        quic.port = quic_port;
        QUIC_ALPN_V1.clone_into(&mut quic.alpn);
        quic.sni_override = None;

        let wss = descriptor
            .carriers
            .wss
            .as_mut()
            .ok_or(ApiError::UnavailableCarrier(CarrierKind::Wss))?;
        wss.url = format!("wss://127.0.0.1:{wss_port}/tunnel");
        WSS_SUBPROTOCOL_V1.clone_into(&mut wss.subprotocol);
        wss.authority_override = None;
        descriptor.resign_with_example_key_for_testing()?;
        descriptor.validate()?;
        *self
            .descriptor
            .lock()
            .map_err(|_| ApiError::TransportSelectorInvariant("descriptor lock poisoned"))? =
            descriptor.clone();
        Ok(descriptor)
    }

    pub fn responder(&self) -> ApiResult<NoiseServiceResponder> {
        let descriptor = self.lock_descriptor()?;
        NoiseServiceResponder::new(descriptor, &self.server_private_key, self.device_public_key)
    }

    pub const fn server_public_key(&self) -> [u8; 32] {
        self.server_public_key
    }

    fn lock_descriptor(&self) -> ApiResult<ServiceDescriptor> {
        self.descriptor
            .lock()
            .map_err(|_| ApiError::TransportSelectorInvariant("descriptor lock poisoned"))
            .map(|descriptor| descriptor.clone())
    }
}

pub fn noise_params() -> ApiResult<snow::params::NoiseParams> {
    NOISE_SUITE_V1
        .parse()
        .map_err(|_| ApiError::InnerNoiseFailure)
}
