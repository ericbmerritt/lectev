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
/// This module provides a command that imports the required data to run a simulation from a set of
/// csv formatted spreadsheets.
use crate::feature_flags;
use crate::lib::simulation::{convert_template, external as sim_external};
use derive_more::Display;
use futures::future;
use percentage_rs::Percentage;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, Snafu};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio_stream::StreamExt;
use tracing::{error, instrument};

#[derive(Debug, Snafu)]
pub enum Error {
    /// Error produced if the data storage JSON blob can't be read from the provided path
    #[snafu(display("Could read JSON blob from {:?}: {}", filename, source))]
    ReadDataFromFile {
        filename: PathBuf,
        source: std::io::Error,
    },
    /// Error produced when a file path that should contain a worker id is empty
    #[snafu(display("Empty file path"))]
    EmptyFilePath,
    /// Error produced when a path is converted to a string and the path can not be represented as
    /// a string
    #[snafu(display("Path can not be represented as a unicode string: {:?}", path))]
    PathCantBeRepresented { path: PathBuf },
    /// Error produced when a worker id can not be created from a string
    #[snafu(display("Worker id can't be created from {}: {}", worker_id, source))]
    WorkerIdCantBeCreated {
        worker_id: String,
        source: sim_external::Error,
    },
    /// Error produced when pto is found for a worker that does not exit.
    #[snafu(display("No estimations for worker: {}", id))]
    NonExistantWorker { id: sim_external::WorkerId },
    /// Error produced when the Pto spreadsheet (csv) can't be opened
    #[snafu(display("Can't open pto file {}: {}", path, source))]
    CantOpenPtoFile {
        path: String,
        source: std::io::Error,
    },
    /// Error produced when the system is unable to read PTO record from PTO sheet
    #[snafu(display("Unable to read Pto record"))]
    UnableToReadPtoRecord,
    /// Error produced if a value can not be deserialized
    #[snafu(display("Unable to read pto value: {}", source))]
    UnableToReadPtoRecordWithError { source: csv_async::Error },
    /// Error produced when the system is unable to read Template record from Template sheet
    #[snafu(display("Unable to read Template record"))]
    UnableToReadTemplateRecord,
    /// Error produced if a value can not be deserialized
    #[snafu(display("Unable to read template value: {}", source))]
    UnableToReadTemplateRecordWithError { source: csv_async::Error },
    #[snafu(display("Feature flag 'SIMULATION_IMPORT' is not enabled"))]
    FeatureFlagNotEnabled,
    /// Could not convert template records to work
    #[snafu(display("Unable to convert csv to templates: {}", source))]
    UnableToConvertTemplateRecords { source: csv_async::Error },
    /// Produced when this module can't convert templates to work
    #[snafu(display("Unable to convert templates to work: {}", source))]
    UnableToConvertTemplatesToWork { source: convert_template::Error },
}

/// Represents holidays as they are defined in the the holiday sheet
#[derive(Display, Deserialize, Serialize)]
#[display(fmt = "Holiday {{description: {}, date: {}}}", description, date)]
struct Holiday {
    description: String,
    date: chrono::NaiveDate,
}

/// Represents the way in which a row of data in the pto sheet is constructed
#[derive(Display, Deserialize, Serialize)]
#[display(
    fmt = "Pto {{worker_id: {}, start_date: {}, end_date: {}, percentage: {}}}",
    worker_id,
    start_date,
    end_date,
    percentage
)]
struct Pto {
    worker_id: String,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
    percentage: Percentage,
}

/// Worker ids are created from the base name of the each estimation sheet. For example, a file
/// with the name `/foo/bar/baz.csv` would identify a worker with the id `baz`.
fn path_to_worker_ids(
    estimation_sheets: &Vec<&Path>,
) -> Result<Vec<sim_external::WorkerId>, Error> {
    let mut result = Vec::with_capacity(estimation_sheets.len());
    for path in estimation_sheets {
        let worker_id_str = path
            .file_stem()
            .context(EmptyFilePath {})?
            .to_str()
            .with_context(|| PathCantBeRepresented {
                path: path.to_path_buf(),
            })?;
        result.push(
            sim_external::WorkerId::new(worker_id_str.to_owned()).with_context(|| {
                WorkerIdCantBeCreated {
                    worker_id: worker_id_str.to_owned(),
                }
            })?,
        )
    }

    Ok(result)
}

/// This function converts a pto object in the csv format into a pto object as needed in the
/// simulation.
fn internal_pto_to_external_pto(pto: &Pto) -> Vec<sim_external::Pto> {
    let mut result = Vec::new();

    let start = pto.start_date;
    while start < pto.end_date {
        result.push(sim_external::Pto {
            date: start,
            percentage: percentage_rs::Percentage::new(100),
        });
        start = start.succ();
    }

    result
}

/// Adds pto to the worker specified in the Pto record, converting all of the pto into the
/// simulation format for Pto. Workers must exit in the `workers` vec or an error is produced.
fn add_pto_to_worker(workers: &mut Vec<sim_external::Worker>, pto: Pto) -> Result<(), Error> {
    let mut all_pto = internal_pto_to_external_pto(&pto);

    let worker_id =
        sim_external::WorkerId::new(pto.worker_id).with_context(|| WorkerIdCantBeCreated {
            worker_id: pto.worker_id,
        })?;

    match workers.iter().find(|worker| worker.id == worker_id) {
        Some(worker) => {
            worker.pto.append(&mut all_pto);
            Ok(())
        }
        None => NonExistantWorker { id: worker_id }.fail(),
    }
}
/// Converts a specific pto sheet to a [`Vec`] of [`Pto`] structs.
#[instrument]
async fn pto_sheet_to_pto(pto_sheet: &Path) -> Result<Vec<Pto>, Error> {
    let mut reader = csv_async::AsyncDeserializer::from_reader(
        tokio::fs::File::open(pto_sheet)
            .await
            .with_context(|| CantOpenPtoFile {
                path: pto_sheet.to_string_lossy(),
            })?,
    );
    let mut pto_records = reader.deserialize::<Pto>();
    let mut result = Vec::new();
    while let pto_record = pto_records.next().await.context(UnableToReadPtoRecord {})? {
        result.push(pto_record.context(UnableToReadPtoRecordWithError {})?);
    }

    Ok(result)
}

/// Estimation sheets should be named as `worker_id`.csv. That allows us to extract the work id
/// from file itself.
#[instrument]
async fn estimations_and_pto_to_workers(
    estimation_sheets: &Vec<&Path>,
    pto_sheets: Vec<&Path>,
) -> Result<Vec<sim_external::Worker>, Error> {
    let mut workers = path_to_worker_ids(estimation_sheets)?
        .into_iter()
        .map(|worker_id| sim_external::Worker {
            id: worker_id,
            pto: Vec::new(),
            skills: HashSet::new(),
        })
        .collect();

    let all_pto: Vec<Pto> = future::try_join_all(pto_sheets.into_iter().map(pto_sheet_to_pto))
        .await?
        .into_iter()
        .flatten()
        .collect();

    for pto in all_pto {
        add_pto_to_worker(&mut workers, pto)?;
    }

    Ok(workers)
}

/// The template is more rigid then the hierarchical work structure that we have.
/// The template has 'rungs', 'tasks' and 'sub_tasks'. Those generally equate to epics, stories
/// and sub_tasks in most peoples thinking. In our simulation structure they equate to two
/// levels of WorkGroup -> WorkGroup -> WorkItem. We allow the user to omit the sub_tasks. If
/// they do that then we end up with WorkGroup -> WorkItem. Either is just fine, we just have
/// to take it into account when 'parsing' the work.
#[instrument]
async fn template_sheet_to_work(template_sheet: &Path) -> Result<Vec<sim_external::Work>, Error> {
    let mut reader = csv_async::AsyncDeserializer::from_reader(
        tokio::fs::File::open(template_sheet)
            .await
            .with_context(|| CantOpenPtoFile {
                path: template_sheet.to_string_lossy(),
            })?,
    );
    let mut template_records = reader.deserialize::<convert_template::Template>();
    let mut resolved_templates = Vec::new();

    while let template_record = template_records
        .next()
        .await
        .context(UnableToReadTemplateRecord {})?
    {
        let template = template_record.context(UnableToReadTemplateRecordWithError {})?;
        resolved_templates.push(template);
    }

    Ok(convert_template::templates_to_work(resolved_templates)
        .context(UnableToConvertTemplatesToWork {})?)
}

#[instrument]
async fn do_command_prime(
    template_sheet: &Path,
    estimations_sheets: Vec<&Path>,
    pto_sheet: Vec<&Path>,
    holiday_sheet: Vec<&Path>,
) -> Result<(), Error> {
    let workers = estimations_and_pto_to_workers(&estimations_sheets, pto_sheet).await?;
    let work = template_sheet_to_work(&template_sheet).await?;

    print!("{}", sim_external::Simulation { work, workers });

    Ok(())
}

#[instrument]
pub async fn do_command(
    template_sheet: &Path,
    estimations_sheets: Vec<&Path>,
    pto_sheet: Vec<&Path>,
    holiday_sheet: Vec<&Path>,
) -> Result<(), Error> {
    if feature_flags::is_enabled(feature_flags::SimulationImport) {
        do_command_prime(template_sheet, estimations_sheets, pto_sheet, holiday_sheet).await?;
        Ok(())
    } else {
        error!("This command is a WIP, you must set the feature flag to continue");
        FeatureFlagNotEnabled.fail()
    }
}
