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
use colored::Colorize;
use snafu::{ResultExt, Snafu};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::instrument;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not read line: {}", source))]
    FailedToReadLine { source: std::io::Error },
    #[snafu(display("Could not write line: {}", source))]
    FailedToWriteLine { source: std::io::Error },
}

#[instrument]
pub async fn write(data: &str) -> Result<(), Error> {
    tokio::io::stdout()
        .write_all(format!("{}\n", data).as_bytes())
        .await
        .context(FailedToWriteLine {})
}

#[instrument]
pub async fn writeln(data: &str) -> Result<(), Error> {
    write(&format!("{}\n", data)).await
}

#[instrument(skip(validator))]
pub async fn get_input(prompt: &str, validator: fn(&str) -> bool) -> Result<Option<String>, Error> {
    for _ in 0..5 {
        write(&format!("{} {} ", prompt.green(), "==>".green())).await?;
        let line = get_line_from_stdin().await?;

        match line {
            None => {
                writeln(&"No input provided".red()).await?;
                continue;
            }
            Some(data) if validator(&data) => return Ok(Some(data)),
            Some(data) => {
                writeln(&format!("'{}' {}", data.red(), " is not valid input".red())).await?;
                continue;
            }
        }
    }
    Ok(None)
}

#[instrument]
pub async fn get_line_from_stdin() -> Result<Option<String>, Error> {
    let reader = BufReader::new(tokio::io::stdin());
    reader
        .lines()
        .next_line()
        .await
        .context(FailedToReadLine {})
}
