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
use crate::feature_flags;
use crate::lib::simulation::external as sim_external;
use crate::lib::simulation::{index, rand_topo};
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use tracing::{error, instrument};

#[derive(Debug, Snafu)]
pub enum Error {
    /// Error produced if the data storage JSON blob can't be read from the provided path
    #[snafu(display("Could read JSON blob from {:?}: {}", filename, source))]
    ReadDataFromFile {
        filename: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Could read JSON blob from stdin: {}", source))]
    ReadDataFromStdin { source: std::io::Error },
    #[snafu(display("Feature flag 'SIMULATION_RUN' is not enabled"))]
    FeatureFlagNotEnabled,
    /// Error produced when the a simulation can't be parsed into valid json.
    #[snafu(display(
        "Could parse the JSON blob from stdin into a simulation struture: {}",
        source
    ))]
    ParsingSimulation { source: serde_json::error::Error },
}

//#[instrument]
//fn simulate<'a>(sim: &index::Indices<'a>) -> sim_external::Projection {}

#[instrument]
async fn get_data(potential_input: &Option<PathBuf>) -> Result<sim_external::Simulation, Error> {
    let data = match potential_input {
        Some(path) => tokio::fs::read_to_string(path.clone())
            .await
            .map_err(|err| Error::ReadDataFromFile {
                filename: path.clone(),
                source: err,
            })?,
        None => {
            let mut buffer = String::new();
            let _ = tokio::io::stdin()
                .read_to_string(&mut buffer)
                .await
                .context(ReadDataFromStdin {})?;
            buffer
        }
    };
    serde_json::from_str::<sim_external::Simulation>(&data).context(ParsingSimulation {})
}

#[instrument]
pub async fn do_command(config_path: &Option<PathBuf>) -> Result<(), Error> {
    if feature_flags::is_enabled(feature_flags::SimulationRun) {
        let simulation = get_data(config_path).await?;
        let indices = index::sim_to_indexes(&simulation);
        let prepared = rand_topo::prepare(&indices);
        print!("{:?}", rand_topo::sort(prepared));
        Ok(())
    } else {
        error!("This command is a WIP, you must set the feature flag to continue");
        FeatureFlagNotEnabled.fail()
    }
}
