extern crate reqwest;
extern crate serde;
extern crate hyper;

use std::result::Result;
use http::hyper::header::{Authorization, Basic};
use serde::de::DeserializeOwned;

pub struct Client {
    client: reqwest::Client,
    auth: Basic,
}

impl Client {
    pub fn new(username: &str, password: &str) -> Client {
        Client {
            client: reqwest::Client::new(),
            auth: Basic {
                username: username.to_string(),
                password: Some(password.to_string()),
            },
        }
    }

    pub fn get<T: DeserializeOwned>(&self, uri: &str) -> Result<T, reqwest::Error> {
        let mut http_response = self.client.get(uri)
            .header(Authorization(self.auth.clone()))
            .send()?;

        let resp: T = http_response.json()?;

        Ok(resp)
    }
}
