// This file is part of Lectev.
//
//  Lectev is free software: you can redistribute it and/or modify
//  it under the terms of the GNU General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.
//
//  Lectev is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU General Public License for more details.
//
//  You should have received a copy of the GNU General Public License
//  along with Lectev.  If not, see <https://www.gnu.org/licenses/>.
//! # Jira Integrations and Enhancements
//!
//! This cli program's primary purpose is to provide enhancements to Jira and allow data extraction
//! without having to go through your Jira administrator or pull something in out of the
//! marketplace. Its also designed so that it could, in the future, interact with other issue
//! tracking systems. Currently nothing by Jira is defined.
#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(
    missing_docs,
    missing_doc_code_examples,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

use serde::Deserialize;
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;
use structopt::StructOpt;
use tracing::{error, info, Level};

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate features;

mod commands {
    pub mod jira;
    pub mod simulation {
        pub mod run;
        pub mod import_csv;
    }
}

mod command;
mod configs {
    pub mod jira;
}
mod config;
mod utils;
mod lib {
    pub mod jira {
        pub mod api;
        pub mod core;
        pub mod native;
        pub mod nativetocore;
        pub mod times_in_flight;
    }
    pub mod rest;
    pub mod simulation {
        pub mod external;
        pub mod index;
        pub mod internal;
        pub mod rand_topo;
        pub mod convert_template;
    }
}

features! {
    mod feature_flags {
        const TimeInStatus = 0b0000_0010,
        const SimulationRun = 0b0000_0100,
        const SimulationImport = 0b0000_1000
    }
}

#[derive(Deserialize, Debug)]
struct Environment {
    /// Enable features that may not be ready for final release. Created as a list of feature
    /// names. Be warned that enabling these features may break things.
    feature_flags: Option<Vec<String>>,
}

/// Provides the errors that this system may produce using [`snafu`].
#[derive(Debug, Snafu)]
pub enum Error {
    /// Produced when a feature flag is specified but that feature flag does not
    /// exist
    #[snafu(display("Invalid feature flag provided: {}", flag))]
    InvalidFeatureFlag {
        /// The unknown flag
        flag: String,
    },
    /// Produced when data can't be extracted from the environment
    #[snafu(display("Couldn't read from environment: {}", source))]
    InvalidEnvironment {
        /// The underlying source of the error
        source: envy::Error,
    },
    /// Produced when the time in status command fails
    #[snafu(display("Failed to run jira time-in-status command: {}", source))]
    FailedToRunJiraTimeInStatus {
        /// The underlying source of the problem in running the command
        source: commands::jira::Error,
    },
    /// Produced when the simulation run command fails
    #[snafu(display("Failed to run simulation run command: {}", source))]
    FailedSimulationRun {
        /// The underlying source of the problem in running the command
        source: commands::simulation::run::Error,
    },
}

#[derive(Debug, StructOpt)]
enum JiraCommand {
    TimeInStatusWip {
        /// Raw api dump file. This dumps the response from jira
        #[structopt(long, parse(from_os_str))]
        debug_jira_file: Option<PathBuf>,

        /// If specified will load from the jira data file specified in the 'debug-jira-file' argument,
        /// and *will not* pull from jira.
        #[structopt(long)]
        load_from_jira_file: bool,
        /// Controls the output of the report. It is *always* in csv format, but you can provide the
        /// path and filename + extension here
        #[structopt(short, long, parse(from_os_str))]
        output_path: PathBuf,
        /// Provides the JQL query that the command uses to gather the Issues which are analyzed
        /// for the Time in Status report.
        #[structopt(short, long)]
        jql_query: String,
    },
}

#[derive(Debug, StructOpt)]
struct Jira {
    // Optional config path for the jira functionality. If not provided the default configuration
    // will be used.
    #[structopt(short, long, parse(from_os_str))]
    config_path: Option<PathBuf>,

    #[structopt(subcommand)]
    cmd: JiraCommand,
}

/// Runs the simulation on the data provided a structure. That structure may come from
/// the provided `input_file` or from `stdin`
#[derive(Debug, StructOpt)]
struct Run {
    /// The input file containing the simulation. This maybe omitted and provided in stdin
    #[structopt(short, long, parse(from_os_str))]
    input_file: Option<PathBuf>,
}

/// Provides the various target commands that run on the simulation
#[derive(Debug, StructOpt)]
enum Simulation {
    Run(Run),
}

#[derive(Debug, StructOpt)]
enum Command {
    Jira(Jira),
    Simulation(Simulation),
}

#[derive(Debug, StructOpt)]
#[structopt(name = "lectev")]
/// The `lectev` command provides supportive tooling for Jira. The coverage
/// that lectev provides is very broad, with each command being independent and unrelated to others.
/// Commands that end in `-wip` are in development and may or map not be usable. To use a command
/// that ends in `-wip` you need to enable the feature. You do that by passing the setting the
/// `LECTEV_FEATURE_FLAGS` environment variable to the name of the command. You may also set it to ALL
/// to enable all feature flags.
struct Opt {
    /// Verbose mode -v 0 = no output, 1 normal output, 2 lots of output
    #[structopt(short, long)]
    verbose: Option<u64>,

    #[structopt(subcommand)]
    command: Command,
}

fn opt_int_to_level(verbosity: &Option<u64>) -> Level {
    match verbosity {
        Some(1) => Level::WARN,
        Some(2) => Level::INFO,
        Some(3) => Level::DEBUG,
        Some(4) => Level::TRACE,
        _ => Level::ERROR,
    }
}

fn enable_feature(feature: &str) -> Result<(), Error> {
    match feature {
        "ALL" => {
            info!("Enabled the all feature flags");
            enable_feature("jira-time-in-status")?;
            enable_feature("simulation-run")?;
            enable_feature("simulation-import")?;
            Ok(())
        }
        "jira-time-in-status" => {
            info!("Enabled the `jira-time-in-status` flag");
            feature_flags::enable(feature_flags::TimeInStatus);
            Ok(())
        }
        "simulation-run" => {
            info!("Enabled the `simulation-run` flag");
            feature_flags::enable(feature_flags::SimulationRun);
            Ok(())
        }
        "simulation-import" => {
            info!("Enable the `simulation-import` flag");
            feature_flags::enable(feature_flags::SimulationImport);
            Ok(())
        }
        _ => {
            error!("Unknown feature flag `{}` specified", feature);
            InvalidFeatureFlag { flag: feature }.fail()
        }
    }
}

fn resolve_features(features_opts: &Option<Vec<String>>) -> Result<(), Error> {
    if let Some(features) = features_opts {
        for feature in features {
            enable_feature(feature)?;
        }
    }

    Ok(())
}

async fn do_jira_reports(config_path: &Option<PathBuf>, cmd: &JiraCommand) -> Result<(), Error> {
    match cmd {
        JiraCommand::TimeInStatusWip {
            debug_jira_file,
            load_from_jira_file,
            output_path,
            jql_query,
        } => commands::jira::do_time_in_status(
            config_path,
            output_path,
            *load_from_jira_file,
            debug_jira_file,
            jql_query,
        )
        .await
        .context(FailedToRunJiraTimeInStatus {}),
    }
}

async fn do_simulation(sim: &Simulation) -> Result<(), Error> {
    match sim {
        Simulation::Run(Run { input_file }) => commands::simulation::run::do_command(input_file)
            .await
            .context(FailedSimulationRun {}),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opt = Opt::from_args();

    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .pretty()
        .with_max_level(opt_int_to_level(&opt.verbose))
        .init();

    let env_config = envy::prefixed("LECTEV_")
        .from_env::<Environment>()
        .context(InvalidEnvironment {})?;

    resolve_features(&env_config.feature_flags)?;

    match opt.command {
        Command::Jira(Jira { config_path, cmd }) => do_jira_reports(&config_path, &cmd).await?,
        Command::Simulation(sim) => do_simulation(&sim).await?,
    }
    Ok(())
}
