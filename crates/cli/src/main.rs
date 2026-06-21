// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Command-line interface for Secure Tunnel.

use std::fmt;
use std::str::FromStr;

use secure_tunnel_harness::{
    ConformanceScenario, ConformanceSuiteReport, SmokeScenario, run_conformance_scenario,
    run_conformance_suite, run_smoke_scenarios,
};

#[tokio::main]
async fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()).await {
        eprintln!("{error}");
        std::process::exit(error.exit_code());
    }
}

async fn run(args: Vec<String>) -> Result<(), CliError> {
    match args.first().map(String::as_str) {
        None => {
            print_metadata();
            Ok(())
        }
        Some("smoke") => run_smoke_command(&args[1..]).await,
        Some("conformance") => run_conformance_command(&args[1..]).await,
        Some("-h" | "--help") => {
            print_usage();
            Ok(())
        }
        Some(_) => Err(CliError::usage("unknown command")),
    }
}

async fn run_smoke_command(args: &[String]) -> Result<(), CliError> {
    let Some(options) = SmokeOptions::parse(args)? else {
        return Ok(());
    };
    let report = run_smoke_scenarios(&options.scenarios).await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

async fn run_conformance_command(args: &[String]) -> Result<(), CliError> {
    let Some(options) = ConformanceOptions::parse(args)? else {
        return Ok(());
    };
    let report = if let Some(scenario) = options.scenario {
        let report = run_conformance_scenario(scenario).await?;
        ConformanceSuiteReport {
            ok: report.ok,
            scenarios: vec![report],
            pending: Vec::new(),
        }
    } else {
        run_conformance_suite().await?
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn print_metadata() {
    let descriptor = secure_tunnel_core::example_service_descriptor();
    println!("secure-tunnel-cli");
    println!("protocol_id: {}", secure_tunnel_core::protocol_id_v1());
    println!("preferred_carrier: quic");
    println!(
        "service: {}.{}",
        descriptor.service_id, descriptor.environment_id
    );
}

fn print_usage() {
    println!("usage:");
    println!("  secure-tunnel-cli");
    println!(
        "  secure-tunnel-cli smoke [--scenario all|quic-success|wss-fallback] [--format json]"
    );
    println!("  secure-tunnel-cli conformance [--scenario all|<name>] [--format json]");
}

struct SmokeOptions {
    scenarios: Vec<SmokeScenario>,
}

impl SmokeOptions {
    fn parse(args: &[String]) -> Result<Option<Self>, CliError> {
        let mut scenario = "all".to_owned();
        let mut format = "json".to_owned();
        let mut index = 0_usize;
        while index < args.len() {
            match args[index].as_str() {
                "-h" | "--help" => {
                    print_usage();
                    return Ok(None);
                }
                "--scenario" => {
                    index += 1;
                    scenario.clone_from(
                        args.get(index)
                            .ok_or(CliError::usage("--scenario requires a value"))?,
                    );
                }
                "--format" => {
                    index += 1;
                    format.clone_from(
                        args.get(index)
                            .ok_or(CliError::usage("--format requires a value"))?,
                    );
                }
                _ => return Err(CliError::usage("unknown smoke option")),
            }
            index += 1;
        }
        if format != "json" {
            return Err(CliError::usage("only --format json is supported"));
        }
        let scenarios = if scenario == "all" {
            vec![SmokeScenario::QuicSuccess, SmokeScenario::WssFallback]
        } else {
            vec![SmokeScenario::from_str(&scenario)?]
        };
        Ok(Some(Self { scenarios }))
    }
}

struct ConformanceOptions {
    scenario: Option<ConformanceScenario>,
}

impl ConformanceOptions {
    fn parse(args: &[String]) -> Result<Option<Self>, CliError> {
        let mut scenario = "all".to_owned();
        let mut format = "json".to_owned();
        let mut index = 0_usize;
        while index < args.len() {
            match args[index].as_str() {
                "-h" | "--help" => {
                    print_usage();
                    return Ok(None);
                }
                "--scenario" => {
                    index += 1;
                    scenario.clone_from(
                        args.get(index)
                            .ok_or(CliError::usage("--scenario requires a value"))?,
                    );
                }
                "--format" => {
                    index += 1;
                    format.clone_from(
                        args.get(index)
                            .ok_or(CliError::usage("--format requires a value"))?,
                    );
                }
                _ => return Err(CliError::usage("unknown conformance option")),
            }
            index += 1;
        }
        if format != "json" {
            return Err(CliError::usage("only --format json is supported"));
        }
        let scenario = if scenario == "all" {
            None
        } else {
            Some(ConformanceScenario::from_str(&scenario)?)
        };
        Ok(Some(Self { scenario }))
    }
}

#[derive(Debug)]
enum CliError {
    Usage(&'static str),
    Harness(secure_tunnel_harness::HarnessError),
    Json(serde_json::Error),
}

impl CliError {
    const fn usage(message: &'static str) -> Self {
        Self::Usage(message)
    }

    const fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            Self::Harness(_) | Self::Json(_) => 1,
        }
    }
}

impl From<secure_tunnel_harness::HarnessError> for CliError {
    fn from(value: secure_tunnel_harness::HarnessError) -> Self {
        Self::Harness(value)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => write!(formatter, "{message}"),
            Self::Harness(error) => write!(formatter, "harness failed: {error}"),
            Self::Json(error) => write!(formatter, "json failed: {error}"),
        }
    }
}

impl std::error::Error for CliError {}
