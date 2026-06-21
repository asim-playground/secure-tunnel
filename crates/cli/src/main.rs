// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Command-line interface for Secure Tunnel.

use std::env;
use std::fmt;
use std::io::Write as _;
use std::path::PathBuf;
use std::str::FromStr;

use secure_tunnel_harness::{
    BindingFixtureReport, ConformanceScenario, ConformanceSuiteReport, SmokeScenario,
    run_binding_fixture_client, run_conformance_scenario, run_conformance_suite,
    run_smoke_scenarios, start_binding_fixture_server,
};

#[tokio::main]
async fn main() {
    init_observability_from_env();
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
        Some("binding-fixture") => run_binding_fixture_command(&args[1..]).await,
        Some("binding-fixture-client") => run_binding_fixture_client_command(&args[1..]).await,
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

async fn run_binding_fixture_command(args: &[String]) -> Result<(), CliError> {
    let Some(_options) = BindingFixtureOptions::parse(args)? else {
        return Ok(());
    };
    let server = start_binding_fixture_server().await?;
    println!("{}", serde_json::to_string(server.report())?);
    std::io::stdout().flush()?;
    tokio::signal::ctrl_c().await?;
    drop(server);
    Ok(())
}

async fn run_binding_fixture_client_command(args: &[String]) -> Result<(), CliError> {
    let Some(options) = BindingFixtureClientOptions::parse(args)? else {
        return Ok(());
    };
    let fixture_json = std::fs::read_to_string(&options.fixture_path)?;
    let fixture: BindingFixtureReport = serde_json::from_str(&fixture_json)?;
    let report = run_binding_fixture_client(&fixture).await?;
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
    println!("  secure-tunnel-cli binding-fixture [--format json]");
    println!("  secure-tunnel-cli binding-fixture-client <fixture-json> [--format json]");
}

fn init_observability_from_env() {
    let enabled = env::var("SECURE_TUNNEL_OBSERVABILITY")
        .is_ok_and(|value| !matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "off"));
    if !enabled {
        return;
    }

    let filter = env::var("RUST_LOG").unwrap_or_else(|_| default_rust_log_filter());
    let filter = tracing_subscriber::EnvFilter::try_new(filter)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_rust_log_filter()));
    let format = env::var("SECURE_TUNNEL_OBSERVABILITY_FORMAT")
        .unwrap_or_else(|_| "compact".to_owned())
        .to_ascii_lowercase();
    let ansi = env::var("SECURE_TUNNEL_OBSERVABILITY_ANSI")
        .is_ok_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"));

    if format == "json" {
        let _ = tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .with_ansi(ansi)
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt()
            .compact()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .with_ansi(ansi)
            .try_init();
    }
}

fn default_rust_log_filter() -> String {
    let level = env::var("SECURE_TUNNEL_OBSERVABILITY_LEVEL").unwrap_or_else(|_| "info".to_owned());
    [
        "secure_tunnel_cli",
        "secure_tunnel_core",
        "secure_tunnel_harness",
        "secure_tunnel_sdk",
        "secure_tunnel_transport",
    ]
    .map(|target| format!("{target}={level}"))
    .join(",")
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

struct BindingFixtureOptions;

impl BindingFixtureOptions {
    fn parse(args: &[String]) -> Result<Option<Self>, CliError> {
        let mut format = "json".to_owned();
        let mut index = 0_usize;
        while index < args.len() {
            match args[index].as_str() {
                "-h" | "--help" => {
                    print_usage();
                    return Ok(None);
                }
                "--format" => {
                    index += 1;
                    format.clone_from(
                        args.get(index)
                            .ok_or(CliError::usage("--format requires a value"))?,
                    );
                }
                _ => return Err(CliError::usage("unknown binding-fixture option")),
            }
            index += 1;
        }
        if format != "json" {
            return Err(CliError::usage("only --format json is supported"));
        }
        Ok(Some(Self))
    }
}

struct BindingFixtureClientOptions {
    fixture_path: PathBuf,
}

impl BindingFixtureClientOptions {
    fn parse(args: &[String]) -> Result<Option<Self>, CliError> {
        let mut fixture_path = None;
        let mut format = "json".to_owned();
        let mut index = 0_usize;
        while index < args.len() {
            match args[index].as_str() {
                "-h" | "--help" => {
                    print_usage();
                    return Ok(None);
                }
                "--format" => {
                    index += 1;
                    format.clone_from(
                        args.get(index)
                            .ok_or(CliError::usage("--format requires a value"))?,
                    );
                }
                value if fixture_path.is_none() => {
                    fixture_path = Some(PathBuf::from(value));
                }
                _ => return Err(CliError::usage("unknown binding-fixture-client option")),
            }
            index += 1;
        }
        if format != "json" {
            return Err(CliError::usage("only --format json is supported"));
        }
        let fixture_path = fixture_path.ok_or(CliError::usage(
            "binding-fixture-client requires a fixture JSON path",
        ))?;
        Ok(Some(Self { fixture_path }))
    }
}

#[derive(Debug)]
enum CliError {
    Usage(&'static str),
    Harness(secure_tunnel_harness::HarnessError),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl CliError {
    const fn usage(message: &'static str) -> Self {
        Self::Usage(message)
    }

    const fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            Self::Harness(_) | Self::Io(_) | Self::Json(_) => 1,
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

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => write!(formatter, "{message}"),
            Self::Harness(error) => write!(formatter, "harness failed: {error}"),
            Self::Io(error) => write!(formatter, "io failed: {error}"),
            Self::Json(error) => write!(formatter, "json failed: {error}"),
        }
    }
}

impl std::error::Error for CliError {}
