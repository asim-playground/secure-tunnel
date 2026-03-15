// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Command-line interface for Secure Tunnel.

fn main() {
    let descriptor = secure_tunnel_core::example_service_descriptor();

    println!("secure-tunnel-cli");
    println!("protocol_id: {}", secure_tunnel_core::protocol_id_v1());
    println!("preferred_carrier: quic");
    println!(
        "service: {}.{}",
        descriptor.service_id, descriptor.environment_id
    );
}
