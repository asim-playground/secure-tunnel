// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::fmt::{self, Write as _};
use std::sync::{Arc, Mutex};

use futures::executor::block_on;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::prelude::*;

use crate::{BootstrapDescriptor, ClientConfig, ConnectOptions, SecureTunnelClient, event_names};

#[test]
fn descriptor_failure_tracing_is_redacted() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let layer = CaptureLayer {
        events: Arc::clone(&events),
    };
    let subscriber = tracing_subscriber::registry().with(layer);
    let client = SecureTunnelClient::new(ClientConfig::default());
    let descriptor = tampered_endpoint_descriptor();

    let result = tracing::subscriber::with_default(subscriber, || {
        block_on(client.connect(ConnectOptions::new(descriptor, 1_742_000_000)))
    });

    assert!(result.is_err());
    let joined = match events.lock() {
        Ok(events) => events,
        Err(poisoned) => poisoned.into_inner(),
    }
    .join("\n");
    assert!(joined.contains(event_names::DESCRIPTOR_VALIDATION));
    for forbidden in [
        "evil.example.com",
        "api.example.com",
        "handshake_hash",
        "service_static_public_key",
        "credential",
        "payload",
    ] {
        assert!(
            !joined.contains(forbidden),
            "trace output leaked forbidden value: {forbidden}\n{joined}"
        );
    }
}

struct CaptureLayer {
    events: Arc<Mutex<Vec<String>>>,
}

impl<S> tracing_subscriber::Layer<S> for CaptureLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let mut visitor = CaptureVisitor::default();
        event.record(&mut visitor);
        match self.events.lock() {
            Ok(mut events) => events.push(visitor.output),
            Err(mut poisoned) => poisoned.get_mut().push(visitor.output),
        }
    }
}

#[derive(Default)]
struct CaptureVisitor {
    output: String,
}

impl Visit for CaptureVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if !self.output.is_empty() {
            self.output.push(' ');
        }
        let _ = write!(self.output, "{}={value:?}", field.name());
    }
}

fn tampered_endpoint_descriptor() -> BootstrapDescriptor {
    let descriptor_json = match BootstrapDescriptor::example_json() {
        Ok(json) => json.replace(
            "\"connect_host\":\"api.example.com\"",
            "\"connect_host\":\"evil.example.com\"",
        ),
        Err(error) => panic!("example descriptor failed: {error}"),
    };
    match BootstrapDescriptor::from_json(&descriptor_json) {
        Ok(descriptor) => descriptor,
        Err(error) => panic!("tampered descriptor failed: {error}"),
    }
}
