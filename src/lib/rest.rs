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

//! Provides a simple wrapper around request. Making it easier to set defaults
//! and reuse them. Specifically `reqwest` has no concept of default credentials. Thats annoying.
//! So we provide this mostly to make it easy to supply default credentials and reuse them in every
//! call rather than spreading them around to every call site.
//!
use base64::write::EncoderWriter as Base64Encoder;
use snafu::{ResultExt, Snafu};
use std::io::Write;
use url::Url;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Invalid username {}: {}", username, source))]
    InvalidUsername {
        username: String,
        source: std::io::Error,
    },
    #[snafu(display("Could not parse password from: {}", source))]
    InvalidPassword { source: std::io::Error },
    #[snafu(display("Could not convert to value: {}", source))]
    InvalidHeaderValue {
        source: reqwest::header::InvalidHeaderValue,
    },
    #[snafu(display("Unable to build reqwest::Client: {}", source))]
    UnableToBuildClient { source: reqwest::Error },
    #[snafu(display("Unable to build url {}: {}", path, source))]
    UnableToBuildUrl {
        path: String,
        source: url::ParseError,
    },
    #[snafu(display("Unable to get request for url {}: {}", path, source))]
    UnableToGetRequestForUrl {
        path: String,
        source: reqwest::Error,
    },
    #[snafu(display("Unable to parse json for url {}: {}", path, source))]
    UnableToParseJsonForUrl {
        path: String,
        source: reqwest::Error,
    },
}
#[derive(Debug)]
pub struct Client {
    base_url: Url,
    client: reqwest::Client,
}

fn basic_auth(username: &str, password: &str) -> Result<reqwest::header::HeaderValue, Error> {
    let mut header_value = b"Basic ".to_vec();
    {
        let mut encoder = Base64Encoder::new(&mut header_value, base64::STANDARD);
        // The unwraps here are fine because Vec::write* is infallible.
        write!(encoder, "{}:", username).context(InvalidUsername { username })?;
        write!(encoder, "{}", password).context(InvalidPassword {})?;
    }

    let encoded_header =
        reqwest::header::HeaderValue::from_bytes(&header_value).context(InvalidHeaderValue {})?;

    Ok(encoded_header)
}
pub fn new(base_url: &Url, username: &str, password: &str) -> Result<Client, Error> {
    let header_value = basic_auth(username, password)?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::AUTHORIZATION, header_value);
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .context(UnableToBuildClient {})?;

    Ok(Client {
        base_url: base_url.clone(),
        client,
    })
}
pub fn get(client: &Client, path: &str) -> Result<reqwest::RequestBuilder, Error> {
    let new_url = client.base_url.join(path).context(UnableToBuildUrl {
        path: path.to_owned(),
    })?;
    Ok(client.client.get(new_url))
}
