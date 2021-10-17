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

use crate::utils;
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Error expanding path to $HOME dir: {}", source))]
    FailedToGetPath {
        source: shellexpand::LookupError<std::env::VarError>,
    },
    #[snafu(display("Could not create directory: {}", source))]
    FailedToCreateDirectory { source: std::io::Error },
    #[snafu(display("Could set restricted permissions in directory: {}", source))]
    CouldntSetRestrictedPermissions { source: utils::Error },
}

pub async fn dir() -> Result<PathBuf, Error> {
    let config_dir_path = PathBuf::from(
        shellexpand::full("~/.config/lectev")
            .context(FailedToGetPath {})?
            .as_ref(),
    );
    tokio::fs::create_dir_all(&config_dir_path)
        .await
        .context(FailedToCreateDirectory {})?;

    utils::set_to_read_write_execute_only_owner(&config_dir_path)
        .await
        .context(CouldntSetRestrictedPermissions {})?;

    Ok(config_dir_path)
}
