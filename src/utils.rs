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
use snafu::{ResultExt, Snafu};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not get metadata for config directory: {}", source))]
    CouldNotGetMetadata { source: std::io::Error },
    #[snafu(display("Could not set permissions for config directory: {}", source))]
    CouldNotSetPermisions { source: std::io::Error },
}
pub async fn set_permissions(config_dir_path: &Path, octal_perms: u32) -> Result<(), Error> {
    let mut perms = tokio::fs::metadata(&config_dir_path)
        .await
        .context(CouldNotGetMetadata {})?
        .permissions();
    perms.set_mode(octal_perms);
    tokio::fs::set_permissions(&config_dir_path, perms)
        .await
        .context(CouldNotSetPermisions {})?;

    Ok(())
}

pub async fn set_to_read_write_execute_only_owner(config_dir_path: &Path) -> Result<(), Error> {
    set_permissions(config_dir_path, 0o700).await
}
