#![deny(missing_docs)]

//! Networking utilities for Rattler, specifically authenticating requests

use std::path::PathBuf;

pub use authentication_storage::{authentication::Authentication, storage::AuthenticationStorage};
use reqwest::{Client, IntoUrl, Method, Url};

pub mod authentication_storage;
pub mod retry_policies;

/// A client that can be used to make authenticated requests, based on the [`reqwest::Client`]
#[derive(Clone)]
pub struct AuthenticatedClient {
    /// The underlying client
    client: Client,

    /// The authentication storage
    auth_storage: AuthenticationStorage,
}

impl Default for AuthenticatedClient {
    fn default() -> Self {
        AuthenticatedClient {
            client: Client::default(),
            auth_storage: AuthenticationStorage::new("rattler", &PathBuf::from("~/.rattler")),
        }
    }
}

impl AuthenticatedClient {
    /// Create a new authenticated client from the given client and authentication storage
    pub fn from_client(client: Client, auth_storage: AuthenticationStorage) -> AuthenticatedClient {
        AuthenticatedClient {
            client,
            auth_storage,
        }
    }
}

impl AuthenticatedClient {
    /// Create a GET request builder for the given URL (see also [`reqwest::Client::get`])
    pub fn get<U: IntoUrl>(&self, url: U) -> reqwest::RequestBuilder {
        self.request(Method::GET, url)
    }

    /// Create a POST request builder for the given URL (see also [`reqwest::Client::post`])
    pub fn post<U: IntoUrl>(&self, url: U) -> reqwest::RequestBuilder {
        self.request(Method::POST, url)
    }

    /// Create a HEAD request builder for the given URL (see also [`reqwest::Client::head`])
    pub fn head<U: IntoUrl>(&self, url: U) -> reqwest::RequestBuilder {
        self.request(Method::HEAD, url)
    }

    /// Create a request builder for the given URL (see also [`reqwest::Client::request`])
    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> reqwest::RequestBuilder {
        let url_clone = url.as_str().to_string();
        match self.auth_storage.get_by_url(url) {
            Err(_) => {
                // forward error to caller (invalid URL)
                self.client.request(method, url_clone)
            }
            Ok((url, auth)) => {
                let url = self.authenticate_url(url, &auth);
                let request_builder = self.client.request(method, url);
                self.authenticate_request(request_builder, &auth)
            }
        }
    }

    /// Authenticate the given URL with the given authentication information
    fn authenticate_url(&self, url: Url, auth: &Option<Authentication>) -> Url {
        if let Some(credentials) = auth {
            match credentials {
                Authentication::CondaToken(token) => {
                    let path = url.path();
                    let mut new_path = String::new();
                    new_path.push_str(format!("/t/{}", token).as_str());
                    new_path.push_str(path);
                    let mut url = url.clone();
                    url.set_path(&new_path);
                    url
                }
                _ => url,
            }
        } else {
            url
        }
    }

    /// Authenticate the given request builder with the given authentication information
    fn authenticate_request(
        &self,
        builder: reqwest::RequestBuilder,
        auth: &Option<Authentication>,
    ) -> reqwest::RequestBuilder {
        if let Some(credentials) = auth {
            match credentials {
                Authentication::BearerToken(token) => builder.bearer_auth(token),
                Authentication::BasicHTTP { username, password } => {
                    builder.basic_auth(username, Some(password))
                }
                Authentication::CondaToken(_) => builder,
            }
        } else {
            builder
        }
    }
}

/// A blocking client that can be used to make authenticated requests, based on the [`reqwest::blocking::Client`]
pub struct AuthenticatedClientBlocking {
    /// The underlying client
    client: reqwest::blocking::Client,

    /// The authentication storage
    auth_storage: AuthenticationStorage,
}

impl AuthenticatedClientBlocking {
    /// Create a new authenticated client from the given client and authentication storage
    pub fn from_client(
        client: reqwest::blocking::Client,
        auth_storage: AuthenticationStorage,
    ) -> AuthenticatedClientBlocking {
        AuthenticatedClientBlocking {
            client,
            auth_storage,
        }
    }
}

impl Default for AuthenticatedClientBlocking {
    fn default() -> Self {
        AuthenticatedClientBlocking {
            client: Default::default(),
            auth_storage: AuthenticationStorage::new("rattler", &PathBuf::from("~/.rattler")),
        }
    }
}

impl AuthenticatedClientBlocking {
    /// Create a GET request builder for the given URL (see also [`reqwest::blocking::Client::get`])
    pub fn get<U: IntoUrl>(&self, url: U) -> reqwest::blocking::RequestBuilder {
        self.request(Method::GET, url)
    }

    /// Create a POST request builder for the given URL (see also [`reqwest::blocking::Client::post`])
    pub fn post<U: IntoUrl>(&self, url: U) -> reqwest::blocking::RequestBuilder {
        self.request(Method::POST, url)
    }

    /// Create a HEAD request builder for the given URL (see also [`reqwest::blocking::Client::head`])
    pub fn head<U: IntoUrl>(&self, url: U) -> reqwest::blocking::RequestBuilder {
        self.request(Method::HEAD, url)
    }

    /// Create a request builder for the given URL (see also [`reqwest::blocking::Client::request`])
    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> reqwest::blocking::RequestBuilder {
        let url_clone = url.as_str().to_string();
        match self.auth_storage.get_by_url(url) {
            Err(_) => {
                // forward error to caller (invalid URL)
                self.client.request(method, url_clone)
            }
            Ok((url, auth)) => {
                let url = self.authenticate_url(url, &auth);
                let request_builder = self.client.request(method, url);
                self.authenticate_request(request_builder, &auth)
            }
        }
    }

    /// Authenticate the given URL with the given authentication information
    fn authenticate_url(&self, url: Url, auth: &Option<Authentication>) -> Url {
        if let Some(credentials) = auth {
            match credentials {
                Authentication::CondaToken(token) => {
                    let path = url.path();
                    let mut new_path = String::new();
                    new_path.push_str(format!("/t/{}", token).as_str());
                    new_path.push_str(path);
                    let mut url = url.clone();
                    url.set_path(&new_path);
                    url
                }
                _ => url,
            }
        } else {
            url
        }
    }

    /// Authenticate the given request builder with the given authentication information
    fn authenticate_request(
        &self,
        builder: reqwest::blocking::RequestBuilder,
        auth: &Option<Authentication>,
    ) -> reqwest::blocking::RequestBuilder {
        if let Some(credentials) = auth {
            match credentials {
                Authentication::BearerToken(token) => builder.bearer_auth(token),
                Authentication::BasicHTTP { username, password } => {
                    builder.basic_auth(username, Some(password))
                }
                Authentication::CondaToken(_) => builder,
            }
        } else {
            builder
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    #[test]
    fn test_store_fallback() -> anyhow::Result<()> {
        let tdir = tempdir()?;
        let storage = super::AuthenticationStorage::new("rattler_test", tdir.path());
        let host = "test.example.com";
        let authentication = Authentication::CondaToken("testtoken".to_string());
        storage.store(host, &authentication)?;
        storage.delete(host)?;
        Ok(())
    }

    #[test]
    fn test_conda_token_storage() -> anyhow::Result<()> {
        let tdir = tempdir()?;
        let storage = super::AuthenticationStorage::new("rattler_test", tdir.path());
        let host = "conda.example.com";
        let retrieved = storage.get(host);

        if let Err(e) = retrieved.as_ref() {
            println!("{:?}", e);
        }

        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_none());

        let authentication = Authentication::CondaToken("testtoken".to_string());
        insta::assert_json_snapshot!(authentication);
        storage.store(host, &authentication)?;

        let retrieved = storage.get(host);
        assert!(retrieved.is_ok());
        let retrieved = retrieved.unwrap();
        assert!(retrieved.is_some());
        let auth = retrieved.unwrap();
        assert!(auth == authentication);

        let client = AuthenticatedClient::from_client(reqwest::Client::default(), storage.clone());
        let request = client.get("https://conda.example.com/conda-forge/noarch/testpkg.tar.bz2");
        let request = request.build().unwrap();
        let url = request.url();

        assert!(url.path().starts_with("/t/testtoken"));

        storage.delete(host)?;
        Ok(())
    }

    #[test]
    fn test_bearer_storage() -> anyhow::Result<()> {
        let tdir = tempdir()?;
        let storage = super::AuthenticationStorage::new("rattler_test", tdir.path());
        let host = "bearer.example.com";
        let retrieved = storage.get(host);

        if let Err(e) = retrieved.as_ref() {
            println!("{:?}", e);
        }

        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_none());

        let authentication = Authentication::BearerToken("xyztokytoken".to_string());

        insta::assert_json_snapshot!(authentication);

        storage.store(host, &authentication)?;

        let retrieved = storage.get(host);
        assert!(retrieved.is_ok());
        let retrieved = retrieved.unwrap();
        assert!(retrieved.is_some());
        let auth = retrieved.unwrap();
        assert!(auth == authentication);

        let client = AuthenticatedClient::from_client(reqwest::Client::default(), storage.clone());
        let request = client.get("https://bearer.example.com/conda-forge/noarch/testpkg.tar.bz2");
        let request = request.build().unwrap();
        let url = request.url();
        assert!(url.to_string() == "https://bearer.example.com/conda-forge/noarch/testpkg.tar.bz2");
        assert_eq!(
            request.headers().get("Authorization").unwrap(),
            "Bearer xyztokytoken"
        );

        storage.delete(host)?;
        Ok(())
    }

    #[test]
    fn test_basic_auth_storage() -> anyhow::Result<()> {
        let tdir = tempdir()?;
        let storage = super::AuthenticationStorage::new("rattler_test", tdir.path());
        let host = "basic.example.com";
        let retrieved = storage.get(host);

        if let Err(e) = retrieved.as_ref() {
            println!("{:?}", e);
        }

        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_none());

        let authentication = Authentication::BasicHTTP {
            username: "testuser".to_string(),
            password: "testpassword".to_string(),
        };
        insta::assert_json_snapshot!(authentication);
        storage.store(host, &authentication)?;

        let retrieved = storage.get(host);
        assert!(retrieved.is_ok());
        let retrieved = retrieved.unwrap();
        assert!(retrieved.is_some());
        let auth = retrieved.unwrap();
        assert!(auth == authentication);

        let client = AuthenticatedClient::from_client(reqwest::Client::default(), storage.clone());
        let request = client.get("https://basic.example.com/conda-forge/noarch/testpkg.tar.bz2");
        let request = request.build().unwrap();
        let url = request.url();
        assert!(url.to_string() == "https://basic.example.com/conda-forge/noarch/testpkg.tar.bz2");
        assert_eq!(
            request.headers().get("Authorization").unwrap(),
            // this is the base64 encoding of "testuser:testpassword"
            "Basic dGVzdHVzZXI6dGVzdHBhc3N3b3Jk"
        );

        storage.delete(host)?;
        Ok(())
    }
}
