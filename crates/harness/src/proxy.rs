// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

use crate::{HarnessResult, map_external};

const MAX_CONNECT_REQUEST_BYTES: usize = 8192;

pub struct HttpProxyServer {
    port: u16,
    connect_authorities: Arc<Mutex<Vec<String>>>,
    task: JoinHandle<()>,
}

impl HttpProxyServer {
    pub async fn start_tunnel() -> HarnessResult<Self> {
        let listener =
            map_external(TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await)?;
        let port = map_external(listener.local_addr())?.port();
        let connect_authorities = Arc::new(Mutex::new(Vec::new()));
        let task_authorities = Arc::clone(&connect_authorities);
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _peer)) = listener.accept().await else {
                    break;
                };
                let authorities = Arc::clone(&task_authorities);
                tokio::spawn(async move {
                    let _ = handle_proxy_client(stream, authorities).await;
                });
            }
        });
        Ok(Self {
            port,
            connect_authorities,
            task,
        })
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn last_connect_authority(&self) -> Option<String> {
        self.connect_authorities.lock().ok()?.last().cloned()
    }
}

impl Drop for HttpProxyServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn handle_proxy_client(
    mut client: TcpStream,
    connect_authorities: Arc<Mutex<Vec<String>>>,
) -> std::io::Result<()> {
    let Some(authority) = read_connect_authority(&mut client).await else {
        return Ok(());
    };
    if let Ok(mut authorities) = connect_authorities.lock() {
        authorities.push(authority.clone());
    }
    let Ok(mut upstream) = TcpStream::connect(authority).await else {
        client
            .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n")
            .await?;
        return Ok(());
    };
    client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;
    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
    Ok(())
}

async fn read_connect_authority(client: &mut TcpStream) -> Option<String> {
    let mut request = Vec::with_capacity(128);
    let mut byte = [0_u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        if request.len() >= MAX_CONNECT_REQUEST_BYTES {
            return None;
        }
        let read = client.read(&mut byte).await.ok()?;
        if read == 0 {
            return None;
        }
        request.push(byte[0]);
    }
    let request = std::str::from_utf8(&request).ok()?;
    let mut parts = request.split("\r\n").next()?.split_whitespace();
    match (parts.next(), parts.next(), parts.next()) {
        (Some("CONNECT"), Some(authority), Some("HTTP/1.1" | "HTTP/1.0")) => {
            Some(authority.to_owned())
        }
        _ => None,
    }
}
