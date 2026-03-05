#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

use reqwest::{
    Client as Http, StatusCode,
    header::{self, HeaderMap, HeaderValue, HeaderName},
};
use serde_json::json;
use std::env;
use types::{Error, Include, InputItemList, Request, Response, ResponseResult};
#[cfg(feature = "stream")]
use {
    async_fn_stream::try_fn_stream,
    futures::{Stream, StreamExt},
    reqwest_eventsource::{Event as EventSourceEvent, RequestBuilderExt},
    types::Event,
};

/// Types for interacting with the Responses API.
pub mod types;

/// The OpenAI Responses API Client.
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    base_url: String,
}

/// Errors that can occur when creating a new Client.
#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    /// The provided API key contains invalid header value characters. Only visible ASCII characters (32-127) are permitted.
    #[error(
        "The provided API key contains invalid header value characters. Only visible ASCII characters (32-127) are permitted."
    )]
    InvalidApiKey,
    /// Failed to create the HTTP Client
    #[error("Failed to create the HTTP Client: {0}")]
    CouldNotCreateClient(#[from] reqwest::Error),
    /// Could not retrieve the ``OPENAI_API_KEY`` env var
    #[error("Could not retrieve the $OPENAI_API_KEY env var")]
    ApiKeyNotFound,
}

#[cfg(feature = "stream")]
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("{0}")]
    Stream(#[from] reqwest_eventsource::Error),
    #[error("Failed to parse event data: {0}")]
    Parsing(#[from] serde_json::Error),
}

/// Builder for constructing a [`Client`] with optional OpenAI specific headers.
///
/// The builder lets you supply the mandatory API key plus the optional
/// `OpenAI-Organization` and `OpenAI-Project` headers in a single fluent
/// chain. If you only need to provide the API key you may continue to use
/// [`Client::new`] or [`Client::from_env`]; the builder is the most flexible
/// entry-point.
///
/// # Examples
/// ```rust
/// use openai_responses::Client;
///
/// let client = Client::builder()
///     .api_key("sk-my-api-key")
///     .organization("org_123")
///     .project("my_project")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    organization: Option<String>,
    project: Option<String>,
}

impl ClientBuilder {
    /// Creates a new [`ClientBuilder`]. This is usually accessed through [`Client::builder`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the OpenAI API key.
    #[must_use]
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Sets the OpenAI organization header (`OpenAI-Organization`).
    #[must_use]
    pub fn organization(mut self, organization: impl Into<String>) -> Self {
        self.organization = Some(organization.into());
        self
    }

    /// Sets the OpenAI project header (`OpenAI-Project`).
    #[must_use]
    pub fn project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Finalises the builder, returning a [`Client`].
    ///
    /// # Errors
    ///
    /// - `CreateError::ApiKeyNotFound` if no API key was provided.
    /// - `CreateError::InvalidApiKey` if any header value contains invalid characters.
    /// - `CreateError::CouldNotCreateClient` if the underlying HTTP client could not be created.
    pub fn build(self) -> Result<Client, CreateError> {
        let api_key = self.api_key.ok_or(CreateError::ApiKeyNotFound)?;

        // Build the default headers
        let mut headers = HeaderMap::from_iter([(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|_| CreateError::InvalidApiKey)?,
        )]);

        if let Some(org) = self.organization {
            headers.insert(
                HeaderName::from_static("openai-organization"),
                HeaderValue::from_str(&org).map_err(|_| CreateError::InvalidApiKey)?,
            );
        }

        if let Some(project) = self.project {
            headers.insert(
                HeaderName::from_static("openai-project"),
                HeaderValue::from_str(&project).map_err(|_| CreateError::InvalidApiKey)?,
            );
        }

        let client = Http::builder().default_headers(headers).build()?;

        Ok(Client { client })
    }
}

impl Client {
    /// Creates a [`ClientBuilder`] for constructing a [`Client`].
    ///
    /// This is the preferred way to initialise a client when you need to set
    /// optional OpenAI headers (organisation / project). If you only require
    /// an API key you can still call [`Client::new`] or [`Client::from_env`].
    ///
    /// ```rust
    /// use openai_responses::Client;
    ///
    /// let client = Client::builder()
    ///     .api_key("sk-...")
    ///     .organization("org_123")
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Creates a new Client with the given API key.
    ///
    /// # Errors
    /// - `CreateError::CouldNotCreateClient` if the HTTP Client could not be created.
    /// - `CreateError::InvalidApiKey` if the API key contains invalid header value characters.
    pub fn new(api_key: &str) -> Result<Self, CreateError> {
        let client = Http::builder()
            .default_headers(HeaderMap::from_iter([(
                header::AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {api_key}"))
                    .map_err(|_| CreateError::InvalidApiKey)?,
            )]))
            .build()?;

        Ok(Self {
            client,
            base_url: "https://api.openai.com".to_owned(),
        })
    }

    /// Set the base URL for the OpenAI API.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Creates a new Client from the `OPENAI_API_KEY` environment variable.
    ///
    /// # Errors
    /// - `CreateError::CouldNotCreateClient` if the HTTP Client could not be created.
    /// - `CreateError::InvalidApiKey` if the API key contains invalid header value characters.
    /// - `CreateError::ApiKeyNotFound` if the `OPENAI_API_KEY` environment variable is not set or contains an equal sign or NUL (`'='` or `'\0'`).
    pub fn from_env() -> Result<Self, CreateError> {
        let api_key = env::var("OPENAI_API_KEY").map_err(|_| CreateError::ApiKeyNotFound)?;

        Self::new(&api_key)
    }

    /// Creates a model response.
    ///
    /// Provide [text](https://platform.openai.com/docs/guides/text) or [image](https://platform.openai.com/docs/guides/images) inputs to generate [text](https://platform.openai.com/docs/guides/text) or [JSON](https://platform.openai.com/docs/guides/structured-outputs) outputs.
    /// Have the model call your own [custom code](https://platform.openai.com/docs/guides/function-calling) or use built-in [tools](https://platform.openai.com/docs/guides/tools) like [web search](https://platform.openai.com/docs/guides/tools-web-search) or [file search](https://platform.openai.com/docs/guides/tools-file-search) to use your own data as input for the model's response.
    /// To receive a stream of tokens as they are generated, use the `stream` function instead.
    ///
    /// ## Errors
    ///
    /// Errors if the request fails to send or has a non-200 status code (except for 400, which will return an OpenAI error instead).
    pub async fn create(
        &self,
        mut request: Request,
    ) -> Result<Result<Response, Error>, reqwest::Error> {
        // Use the `stream` function to stream the response.
        request.stream = Some(false);

        let mut response = self
            .client
            .post(format!("{}/v1/responses", self.base_url))
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::BAD_REQUEST {
            response = response.error_for_status()?;
        }

        response.json::<ResponseResult>().await.map(Into::into)
    }

    #[cfg(feature = "stream")]
    /// Creates a model response and streams it back as it is generated.
    ///
    /// Provide [text](https://platform.openai.com/docs/guides/text) or [image](https://platform.openai.com/docs/guides/images) inputs to generate [text](https://platform.openai.com/docs/guides/text) or [JSON](https://platform.openai.com/docs/guides/structured-outputs) outputs.
    /// Have the model call your own [custom code](https://platform.openai.com/docs/guides/function-calling) or use built-in [tools](https://platform.openai.com/docs/guides/tools) like [web search](https://platform.openai.com/docs/guides/tools-web-search) or [file search](https://platform.openai.com/docs/guides/tools-file-search) to use your own data as input for the model's response.
    ///
    /// To receive the response as a regular HTTP response, use the `create` function.
    pub fn stream(
        &self,
        mut request: Request,
    ) -> impl Stream<Item = Result<Event, StreamError>> + Send + 'static + use<'_> {
        // Use the `create` function to receive a regular HTTP response.
        request.stream = Some(true);

        let mut event_source = self
            .client
            .post(format!("{}/v1/responses", self.base_url))
            .json(&request)
            .eventsource()
            .unwrap_or_else(|_| unreachable!("Body is never a stream"));

        let stream = try_fn_stream(|emitter| async move {
            while let Some(event) = event_source.next().await {
                let message = match event {
                    Ok(EventSourceEvent::Open) => continue,
                    Ok(EventSourceEvent::Message(message)) => message,
                    Err(error) => {
                        if matches!(error, reqwest_eventsource::Error::StreamEnded) {
                            break;
                        }

                        emitter.emit_err(StreamError::Stream(error)).await;
                        continue;
                    }
                };

                match serde_json::from_str::<Event>(&message.data) {
                    Ok(event) => emitter.emit(event).await,
                    Err(error) => emitter.emit_err(StreamError::Parsing(error)).await,
                }
            }

            Ok(())
        });

        Box::pin(stream)
    }

    /// Retrieves a model response with the given ID.
    ///
    /// ## Errors
    ///
    /// Errors if the request fails to send or has a non-200 status code (except for 400, which will return an OpenAI error instead).
    pub async fn get(
        &self,
        response_id: &str,
        include: Option<Include>,
    ) -> Result<Result<Response, Error>, reqwest::Error> {
        let mut response = self
            .client
            .get(format!("{}/v1/responses/{response_id}", self.base_url))
            .query(&json!({ "include": include }))
            .send()
            .await?;

        if response.status() != StatusCode::BAD_REQUEST {
            response = response.error_for_status()?;
        }

        response.json::<ResponseResult>().await.map(Into::into)
    }

    /// Deletes a model response with the given ID.
    ///
    /// ## Errors
    ///
    /// Errors if the request fails to send or has a non-200 status code.
    pub async fn delete(&self, response_id: &str) -> Result<(), reqwest::Error> {
        self.client
            .delete(format!("{}/v1/responses/{response_id}", self.base_url))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Returns a list of input items for a given response.
    ///
    /// ## Errors
    ///
    /// Errors if the request fails to send or has a non-200 status code.
    pub async fn list_inputs(&self, response_id: &str) -> Result<InputItemList, reqwest::Error> {
        self.client
            .get(format!(
                "{}/v1/responses/{response_id}/inputs",
                self.base_url
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_without_api_key_fails() {
        let err = Client::builder().build().unwrap_err();
        assert!(matches!(err, CreateError::ApiKeyNotFound));
    }

    #[test]
    fn builder_rejects_invalid_header_values() {
        let result = Client::builder()
            .api_key("sk-test")
            .organization("org\nnewline")
            .build();
        assert!(matches!(result.unwrap_err(), CreateError::InvalidApiKey));
    }

    #[tokio::test]
    async fn builder_sends_all_headers_over_wire() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        use wiremock::matchers::{header, method, path};

        // Start an ephemeral server.
        let server = MockServer::start().await;

        // Register expectation: we should receive all headers.
        Mock::given(method("GET"))
            .and(path("/"))
            .and(header("authorization", "Bearer sk-test"))
            .and(header("openai-organization", "my-org"))
            .and(header("openai-project", "my-proj"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        // Build our client with all optional headers.
        let client = Client::builder()
            .api_key("sk-test")
            .organization("my-org")
            .project("my-proj")
            .build()
            .unwrap();

        // Make a simple GET request to the mock server.
        let resp = client
            .client
            .get(server.uri())
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
    }
}
