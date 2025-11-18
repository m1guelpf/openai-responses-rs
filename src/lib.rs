#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::doc_markdown)]
#![doc = include_str!("../README.md")]

use base64::Engine;
use log::trace;
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client as Http, StatusCode,
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

/// Re-export the derive macro so users only need `openai_responses` as a dependency.
pub use openai_responses_derive::OpenAiFunction;

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

    #[error("HTTP error: {status}. Body: {body}")]
    Http {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Invalid content type: {content_type:?}. Body: {body}")]
    InvalidContentType {
        content_type: reqwest::header::HeaderValue,
        body: String,
    },
    #[error("Failed to parse event data: {0}")]
    Parsing(#[from] serde_json::Error),
    #[error("Failed to parse event data: {0}")]
    ParsingData(String, serde_json::Error),
}

async fn read_body_safely(response: reqwest::Response) -> String {
    const MAX_BYTES: usize = 16 * 1024;
    let ct = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let prefer_text =
        ct.contains("text/") || ct.contains("json") || ct.contains("xml") || ct.contains("html");

    if prefer_text {
        match response.text().await {
            Ok(mut s) => {
                if s.len() > MAX_BYTES {
                    s.truncate(MAX_BYTES);
                    s.push_str("… [truncated]");
                }
                s
            }
            Err(e) => format!("<<failed to read body as text: {}>>", e),
        }
    } else {
        match response.bytes().await {
            Ok(bytes) => {
                let slice = &bytes[..bytes.len().min(MAX_BYTES)];
                match std::str::from_utf8(slice) {
                    Ok(s) if !s.trim().is_empty() => {
                        let mut s = s.to_string();
                        if bytes.len() > MAX_BYTES {
                            s.push_str("… [truncated]");
                        }
                        s
                    }
                    _ => {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(slice);
                        if bytes.len() > MAX_BYTES {
                            format!(
                                "<<{} bytes binary (base64, truncated)>> {}",
                                bytes.len(),
                                b64
                            )
                        } else {
                            format!("<<{} bytes binary (base64)>> {}", bytes.len(), b64)
                        }
                    }
                }
            }
            Err(e) => format!("<<failed to read body as bytes: {}>>", e),
        }
    }
}

impl Client {
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
                    Err(reqwest_eventsource::Error::InvalidStatusCode(status, response)) => {
                        let body = read_body_safely(response).await;
                        trace!("InvalidStatusCode error: {:#?}", body);
                        emitter.emit_err(StreamError::Http { status, body }).await;
                        break; // stop or retry per your policy
                    }
                    Err(reqwest_eventsource::Error::InvalidContentType(header, response)) => {
                        let body = read_body_safely(response).await;
                        trace!("InvalidContentType error: {:#?}", body);
                        emitter
                            .emit_err(StreamError::InvalidContentType {
                                content_type: header,
                                body,
                            })
                            .await;
                        break; // stop or retry per your policy
                    }

                    Err(e @ reqwest_eventsource::Error::Transport(_)) => {
                        trace!("Transport error: {:#?}", e);
                        emitter.emit_err(StreamError::Stream(e)).await;
                        break; // stop consuming – the EventSource will be dropped
                    }
                    Err(error) => {
                        if matches!(error, reqwest_eventsource::Error::StreamEnded) {
                            trace!("Stream ended: {:#?}", error);
                            break;
                        }

                        emitter.emit_err(StreamError::Stream(error)).await;
                        continue;
                    }
                };
                // TODO??? Handle [DONE]

                match serde_json::from_str::<Event>(&message.data) {
                    Ok(event) => {
                        match &event {
                            Event::Error {
                                message,
                                code,
                                param,
                            } => {
                                trace!(
                                    "Received error line: {} (code: {:#?}, param: {:#?})",
                                    message, code, param
                                );
                            }
                            _ => {}
                        }
                        emitter.emit(event).await
                    }
                    Err(error) => {
                        trace!(
                            "Serde parsing error: {:#?}.\n Message was: {:#?}",
                            error, message.data
                        );
                        emitter
                            .emit_err(StreamError::ParsingData(message.data, error))
                            .await
                    }
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

/* -------------------------------------------------------------------------
   Helper: build `Tool` specs automatically from Rust types
   --------------------------------------------------------------------- */

#[cfg(feature = "schema")]
pub mod tool_builder {
    //! Helpers to generate an OpenAI *Tool* definition from a Rust structure.
    //!
    //! The implementation relies on the `schemars` crate to obtain a JSON
    //! Schema for the Rust type.  If you use the `OpenAiFunction` derive macro
    //! (`openai_responses_derive`) the macro will inject the necessary `NAME`
    //! and `DESCRIPTION` constants automatically – otherwise you can implement
    //! the [`IntoFunction`] trait manually.

    use schemars::JsonSchema;
    use serde::de::DeserializeOwned;
    use serde_json::Value;

    use crate::types::Function;

    /// Recursively processes a JSON schema to replace oneOf with anyOf and add additionalProperties: false to all objects
    /// Additionally, when encountering anyOf branches, ensures each branch that defines properties has a
    /// `required` array listing all property keys, as required by OpenAI's function schema validation.
    fn process_schema_recursively(value: &mut Value) {
        match value {
            Value::Object(map) => {
                // Replace oneOf with anyOf
                if let Some(one_of) = map.remove("oneOf") {
                    map.insert("anyOf".to_string(), one_of);
                }

                // If we have an anyOf array, ensure each branch with properties declares all of them as required
                if let Some(Value::Array(any_of_arr)) = map.get_mut("anyOf") {
                    for branch in any_of_arr.iter_mut() {
                        if let Value::Object(branch_map) = branch {
                            // Prefer explicit object type, but even without it, if properties exist we treat as object
                            let has_properties = branch_map
                                .get("properties")
                                .and_then(Value::as_object)
                                .is_some();

                            if has_properties {
                                // Ensure additionalProperties is false for object branches
                                branch_map
                                    .entry("additionalProperties".to_string())
                                    .or_insert_with(|| Value::Bool(false));

                                // Build required = all property keys
                                if let Some(props) =
                                    branch_map.get("properties").and_then(Value::as_object)
                                {
                                    let mut required_keys: Vec<Value> =
                                        props.keys().map(|k| Value::String(k.clone())).collect();
                                    // Keep stable order for determinism
                                    required_keys.sort_by(|a, b| {
                                        a.as_str().unwrap_or("").cmp(b.as_str().unwrap_or(""))
                                    });
                                    branch_map.insert(
                                        "required".to_string(),
                                        Value::Array(required_keys),
                                    );
                                }
                            }
                        }
                    }
                }

                // Add additionalProperties: false to all object types
                if map.contains_key("type") {
                    if let Some(Value::String(type_str)) = map.get("type") {
                        if type_str == "object" {
                            map.entry("additionalProperties".to_string())
                                .or_insert_with(|| Value::Bool(false));
                        }
                    }
                }

                // Also handle objects that don't have explicit type but have properties
                if map.contains_key("properties") && !map.contains_key("type") {
                    map.entry("additionalProperties".to_string())
                        .or_insert_with(|| Value::Bool(false));
                }

                // Recursively process all values in the object
                for (_, v) in map.iter_mut() {
                    process_schema_recursively(v);
                }
            }
            Value::Array(arr) => {
                // Recursively process all items in arrays
                for item in arr.iter_mut() {
                    process_schema_recursively(item);
                }
            }
            _ => {
                // For primitive values, no processing needed
            }
        }
    }

    // Inline local JSON Schema $ref that point into the root (e.g., #/$defs/* or #/definitions/*)
    fn inline_local_refs(value: &mut Value) {
        fn resolve_pointer<'a>(root: &'a Value, ptr: &str) -> Option<&'a Value> {
            if !ptr.starts_with("#/") {
                return None;
            }
            let mut cur = root;
            for token in ptr.trim_start_matches("#/").split('/') {
                if let Value::Object(map) = cur {
                    cur = map.get(token)?;
                } else {
                    return None;
                }
            }
            Some(cur)
        }

        fn recurse(node: &mut Value, root: &Value, stack: &mut Vec<String>) {
            match node {
                Value::Object(map) => {
                    if let Some(Value::String(r)) = map.get("$ref").cloned() {
                        if r.starts_with("#/") {
                            if stack.contains(&r) {
                                // Cycle detected, keep the $ref to avoid infinite loop
                            } else if let Some(target) = resolve_pointer(root, &r) {
                                stack.push(r.clone());
                                *node = target.clone();
                                recurse(node, root, stack);
                                stack.pop();
                                return;
                            }
                        }
                    }
                    for v in map.values_mut() {
                        recurse(v, root, stack);
                    }
                }
                Value::Array(arr) => {
                    for it in arr.iter_mut() {
                        recurse(it, root, stack);
                    }
                }
                _ => {}
            }
        }

        let root_snapshot = value.clone();
        // Preserve original defs/definitions from the root snapshot so we can reattach them if needed
        let saved_defs = match &root_snapshot {
            Value::Object(m) => m.get("$defs").cloned(),
            _ => None,
        };
        let saved_definitions = match &root_snapshot {
            Value::Object(m) => m.get("definitions").cloned(),
            _ => None,
        };

        let mut stack: Vec<String> = Vec::new();
        recurse(value, &root_snapshot, &mut stack);

        // Only remove definition containers if no local $ref remain
        fn has_local_ref(v: &Value) -> bool {
            match v {
                Value::Object(m) => {
                    if let Some(Value::String(r)) = m.get("$ref") {
                        if r.starts_with("#/") {
                            return true;
                        }
                    }
                    m.values().any(has_local_ref)
                }
                Value::Array(a) => a.iter().any(has_local_ref),
                _ => false,
            }
        }

        if !has_local_ref(value) {
            if let Value::Object(map) = value {
                map.remove("$defs");
                map.remove("definitions");
            }
        } else {
            // Local refs remain (e.g., recursive schemas). Ensure root keeps defs for resolvability.
            if let Value::Object(map) = value {
                if map.get("$defs").is_none() {
                    if let Some(v) = saved_defs {
                        map.insert("$defs".to_string(), v);
                    }
                }
                if map.get("definitions").is_none() {
                    if let Some(v) = saved_definitions {
                        map.insert("definitions".to_string(), v);
                    }
                }
            }
        }
    }

    // Flatten unions (anyOf/oneOf) into a single object with merged, optional properties
    fn flatten_unions(value: &mut Value) {
        fn merge_property_schema(existing: Option<Value>, new_schema: Value) -> Value {
            match existing {
                None => new_schema,
                Some(prev) => {
                    if prev == new_schema {
                        prev
                    } else {
                        // Create a union of both schemas to be permissive
                        Value::Object(serde_json::Map::from_iter([(
                            "anyOf".to_string(),
                            Value::Array(vec![prev, new_schema]),
                        )]))
                    }
                }
            }
        }

        match value {
            Value::Object(map) => {
                // Handle both anyOf and oneOf (oneOf may appear if not normalized yet)
                let union_key = if map.contains_key("anyOf") {
                    Some("anyOf")
                } else if map.contains_key("oneOf") {
                    Some("oneOf")
                } else {
                    None
                };

                if let Some(key) = union_key {
                    if let Some(Value::Array(mut variants)) = map.remove(key) {
                        // Start from existing properties if any
                        let mut merged_props: serde_json::Map<String, Value> = map
                            .get("properties")
                            .and_then(|v| v.as_object())
                            .cloned()
                            .unwrap_or_default();
                        // Track in which variant names a given property appears
                        let mut prop_variants: std::collections::HashMap<String, Vec<String>> =
                            std::collections::HashMap::new();

                        // We'll flatten unions inside each variant before attempting merge
                        for br in variants.iter_mut() {
                            flatten_unions(br);
                        }

                        // Determine discriminant field generically: a string property with distinct const values across all variants
                        // First, collect for each variant a map of property -> const string value (or single-value enum)
                        let mut per_variant_consts: Vec<std::collections::HashMap<String, String>> =
                            Vec::new();
                        for br in variants.iter() {
                            let mut mapc: std::collections::HashMap<String, String> =
                                std::collections::HashMap::new();
                            if let Value::Object(br_map) = br {
                                if let Some(props) =
                                    br_map.get("properties").and_then(|v| v.as_object())
                                {
                                    for (pk, ps) in props.iter() {
                                        if let Some(pmap) = ps.as_object() {
                                            // must be string type (if specified) and have const or enum single value
                                            let type_is_string = match pmap.get("type") {
                                                Some(Value::String(t)) => t == "string",
                                                _ => true, // allow missing type if const is string
                                            };
                                            if type_is_string {
                                                if let Some(Value::String(cv)) = pmap.get("const") {
                                                    mapc.insert(pk.clone(), cv.clone());
                                                } else if let Some(Value::Array(arr)) =
                                                    pmap.get("enum")
                                                {
                                                    if arr.len() == 1 {
                                                        if let Some(Value::String(cv)) = arr.first()
                                                        {
                                                            mapc.insert(pk.clone(), cv.clone());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            per_variant_consts.push(mapc);
                        }
                        // Find a property present with distinct values in every variant
                        let mut chosen_discriminant: Option<(String, Vec<String>)> = None;
                        if !per_variant_consts.is_empty() {
                            // gather candidate keys: intersection of all maps
                            let mut candidates: Option<std::collections::HashSet<String>> = None;
                            for m in &per_variant_consts {
                                let keys: std::collections::HashSet<String> =
                                    m.keys().cloned().collect();
                                candidates = Some(if let Some(acc) = candidates {
                                    acc.intersection(&keys).cloned().collect()
                                } else {
                                    keys
                                });
                            }
                            if let Some(cands) = candidates {
                                'outer: for key in cands {
                                    let mut vals: Vec<String> = Vec::new();
                                    for m in &per_variant_consts {
                                        if let Some(v) = m.get(&key) {
                                            vals.push(v.clone());
                                        } else {
                                            continue 'outer;
                                        }
                                    }
                                    // ensure all distinct
                                    let mut set = std::collections::HashSet::new();
                                    if vals.iter().all(|v| set.insert(v.clone())) {
                                        chosen_discriminant = Some((key, vals));
                                        break;
                                    }
                                }
                            }
                        }
                        // Build variant labels
                        let mut variant_labels: Vec<String> = Vec::new();
                        match &chosen_discriminant {
                            Some((_key, vals)) => {
                                variant_labels = vals.clone();
                            }
                            None => {
                                for idx in 0..variants.len() {
                                    variant_labels.push((idx + 1).to_string());
                                }
                            }
                        }

                        for (idx, br) in variants.iter().enumerate() {
                            if let Value::Object(br_map_initial) = br {
                                let mut br_map = br_map_initial.clone();
                                if let Some(Value::Object(props)) = br_map.remove("properties") {
                                    for (k, v) in props.into_iter() {
                                        let prev = merged_props.remove(k.as_str());
                                        let merged = merge_property_schema(prev, v);
                                        merged_props.insert(k.clone(), merged);
                                        prop_variants
                                            .entry(k)
                                            .or_default()
                                            .push(variant_labels[idx].clone());
                                    }
                                }
                            }
                        }

                        if !merged_props.is_empty() {
                            // Annotate each property's description with variant membership
                            for (k, v) in merged_props.iter_mut() {
                                if let Some(variants) = prop_variants.get(k) {
                                    let mut lst = variants.clone();
                                    lst.sort_unstable();
                                    lst.dedup();
                                    let label = match &chosen_discriminant {
                                        Some((disc_key, _)) => format!(
                                            "Valid when {} is: {}",
                                            disc_key,
                                            lst.join(", ")
                                        ),
                                        None => {
                                            format!("Valid in union variants: {}", lst.join(", "))
                                        }
                                    };
                                    match v {
                                        Value::Object(pmap) => match pmap.get_mut("description") {
                                            Some(Value::String(s)) => {
                                                let already = s.contains("Valid when ")
                                                    || s.contains("Valid in union variants:");
                                                if !already {
                                                    let combined = format!("{} [{}]", s, label);
                                                    pmap.insert(
                                                        "description".to_string(),
                                                        Value::String(combined),
                                                    );
                                                }
                                            }
                                            _ => {
                                                pmap.insert(
                                                    "description".to_string(),
                                                    Value::String(label),
                                                );
                                            }
                                        },
                                        _ => {}
                                    }
                                }
                            }

                            map.insert("type".to_string(), Value::String("object".to_string()));
                            map.insert("properties".to_string(), Value::Object(merged_props));
                            // Remove any strictness from required; everything optional after flattening
                            map.remove("required");
                            map.entry("additionalProperties".to_string())
                                .or_insert(Value::Bool(false));
                            // Tag description
                            let note = "Note: union flattened; fields optional";
                            match map.get_mut("description") {
                                Some(Value::String(s)) => {
                                    if !s.contains(note) {
                                        let combined = format!("{} [{}]", s, note);
                                        map.insert(
                                            "description".to_string(),
                                            Value::String(combined),
                                        );
                                    }
                                }
                                _ => {
                                    map.insert(
                                        "description".to_string(),
                                        Value::String(note.to_string()),
                                    );
                                }
                            }
                        } else {
                            // Could not flatten (e.g., primitive union). Restore the original union key/variants.
                            map.insert(key.to_string(), Value::Array(variants));
                        }
                    }
                }

                // Recurse into the rest
                for v in map.values_mut() {
                    flatten_unions(v);
                }
            }
            Value::Array(arr) => {
                for it in arr.iter_mut() {
                    flatten_unions(it);
                }
            }
            _ => {}
        }
    }

    // Collapse unions of const string branches into a single string enum
    fn collapse_const_string_unions(value: &mut Value) {
        fn collect_string_consts(node: &Value, out: &mut Vec<String>) -> bool {
            match node {
                Value::Object(map) => {
                    // Direct const string
                    if let Some(Value::String(s)) = map.get("const") {
                        // Ensure declared type is compatible with string if present
                        if let Some(tval) = map.get("type") {
                            match tval {
                                Value::String(t) => {
                                    if t != "string" {
                                        return false;
                                    }
                                }
                                Value::Array(arr) => {
                                    // At least one of the types must be "string"
                                    let mut has_string = false;
                                    for t in arr {
                                        if let Some(ts) = t.as_str() {
                                            if ts == "string" {
                                                has_string = true;
                                            }
                                        }
                                    }
                                    if !has_string {
                                        return false;
                                    }
                                }
                                _ => {}
                            }
                        }
                        out.push(s.clone());
                        true
                    } else if let Some(Value::Array(arr)) = map.get("anyOf") {
                        for v in arr {
                            if !collect_string_consts(v, out) {
                                return false;
                            }
                        }
                        true
                    } else if let Some(Value::Array(arr)) = map.get("oneOf") {
                        for v in arr {
                            if !collect_string_consts(v, out) {
                                return false;
                            }
                        }
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }

        match value {
            Value::Object(map) => {
                let union_key = if map.contains_key("anyOf") {
                    Some("anyOf")
                } else if map.contains_key("oneOf") {
                    Some("oneOf")
                } else {
                    None
                };
                if let Some(key) = union_key {
                    if let Some(arr) = map.get(key) {
                        if let Some(branches) = arr.as_array() {
                            let mut vals: Vec<String> = Vec::new();
                            let mut ok = true;
                            for b in branches {
                                if !collect_string_consts(b, &mut vals) {
                                    ok = false;
                                    break;
                                }
                            }
                            if ok && !vals.is_empty() {
                                vals.sort();
                                vals.dedup();
                                // replace union with string enum
                                map.remove(key);
                                map.insert("type".to_string(), Value::String("string".to_string()));
                                map.insert(
                                    "enum".to_string(),
                                    Value::Array(vals.into_iter().map(Value::String).collect()),
                                );
                            }
                        }
                    }
                }
                // Recurse into children regardless
                for v in map.values_mut() {
                    collapse_const_string_unions(v);
                }
            }
            Value::Array(arr) => {
                for it in arr.iter_mut() {
                    collapse_const_string_unions(it);
                }
            }
            _ => {}
        }
    }

    // Final pass: ensure every object has required listing all properties and that fields that were optional
    // before this pass accept null. This reduces model confusion by making optionality explicit via nullability.
    pub(crate) fn enforce_full_required_and_nullable(value: &mut Value) {
        use serde_json::Map;

        fn map_has_type_null(map: &Map<String, Value>) -> bool {
            if let Some(t) = map.get("type") {
                match t {
                    Value::String(s) => s == "null",
                    Value::Array(arr) => arr.iter().any(|x| x.as_str() == Some("null")),
                    _ => false,
                }
            } else {
                false
            }
        }

        fn union_has_null(map: &Map<String, Value>, key: &str) -> bool {
            if let Some(Value::Array(arr)) = map.get(key) {
                for b in arr {
                    if let Value::Object(mb) = b {
                        if map_has_type_null(mb) {
                            return true;
                        }
                        if let Some(Value::Null) = mb.get("const") {
                            return true;
                        }
                    }
                }
            }
            false
        }

        fn schema_is_nullable(map: &Map<String, Value>) -> bool {
            if map_has_type_null(map) {
                return true;
            }
            if union_has_null(map, "anyOf") || union_has_null(map, "oneOf") {
                return true;
            }
            if let Some(Value::Array(en)) = map.get("enum") {
                if en.iter().any(|v| v.is_null()) {
                    return true;
                }
            }
            if let Some(v) = map.get("const") {
                if v.is_null() {
                    return true;
                }
            }
            false
        }

        fn ensure_nullable(node: &mut Value) {
            match node {
                Value::Object(map) => {
                    if schema_is_nullable(map) {
                        return;
                    }

                    if let Some(t) = map.get_mut("type") {
                        match t {
                            Value::String(s) => {
                                if s.as_str() != "null" {
                                    *t = Value::Array(vec![
                                        Value::String(s.clone()),
                                        Value::String("null".to_string()),
                                    ]);
                                }
                                return;
                            }
                            Value::Array(arr) => {
                                let has_null = arr.iter().any(|x| x.as_str() == Some("null"));
                                if !has_null {
                                    arr.push(Value::String("null".to_string()));
                                }
                                return;
                            }
                            _ => {}
                        }
                    }

                    if let Some(Value::Array(arr)) = map.get_mut("anyOf") {
                        // add null branch if not present
                        let present = arr.iter().any(|b| {
                            b.get("type").and_then(Value::as_str) == Some("null")
                                || b.get("type")
                                    .and_then(Value::as_array)
                                    .map(|a| a.iter().any(|x| x.as_str() == Some("null")))
                                    .unwrap_or(false)
                        });
                        if !present {
                            arr.push(serde_json::json!({"type":"null"}));
                        }
                        return;
                    }
                    if let Some(Value::Array(arr)) = map.get_mut("oneOf") {
                        let present = arr.iter().any(|b| {
                            b.get("type").and_then(Value::as_str) == Some("null")
                                || b.get("type")
                                    .and_then(Value::as_array)
                                    .map(|a| a.iter().any(|x| x.as_str() == Some("null")))
                                    .unwrap_or(false)
                        });
                        if !present {
                            arr.push(serde_json::json!({"type":"null"}));
                        }
                        return;
                    }

                    // Fallback: wrap with anyOf including null
                    let clone = Value::Object(map.clone());
                    *node = serde_json::json!({ "anyOf": [ clone, {"type":"null"} ] });
                }
                _ => {}
            }
        }

        match value {
            Value::Object(map) => {
                // If this schema has properties, process optionality based on current required
                if map.get("properties").and_then(Value::as_object).is_some() {
                    // Capture original required BEFORE mutably borrowing properties
                    let mut required_set: std::collections::HashSet<String> =
                        std::collections::HashSet::new();
                    if let Some(Value::Array(req)) = map.get("required") {
                        for r in req {
                            if let Some(s) = r.as_str() {
                                required_set.insert(s.to_string());
                            }
                        }
                    }

                    // Now borrow properties mutably
                    if let Some(Value::Object(props)) = map.get_mut("properties") {
                        // Update each property schema
                        for (k, v) in props.iter_mut() {
                            let was_required = required_set.contains(k);
                            if !was_required {
                                ensure_nullable(v);
                            }
                            // Recurse into property schema for nested objects
                            enforce_full_required_and_nullable(v);
                        }

                        // Set required to all property keys (sorted)
                        let mut keys: Vec<String> = props.keys().cloned().collect();
                        keys.sort();
                        map.insert(
                            "required".to_string(),
                            Value::Array(keys.into_iter().map(Value::String).collect()),
                        );
                    }

                    map.entry("additionalProperties".to_string())
                        .or_insert(Value::Bool(false));
                }

                // Recurse into other fields (except the properties map itself which we handled)
                for (k, v) in map.iter_mut() {
                    if k != "properties" {
                        enforce_full_required_and_nullable(v);
                    }
                }
            }
            Value::Array(arr) => {
                for it in arr.iter_mut() {
                    enforce_full_required_and_nullable(it);
                }
            }
            _ => {}
        }
    }

    /// Remove unknown fields from JSON according to an object schema (best-effort based on properties/anyOf/oneOf).
    pub fn prune_unknown_fields_by_schema(value: &mut Value, schema: &Value) {
        use serde_json::Map;
        fn is_object_like(map: &Map<String, Value>) -> bool {
            map.get("properties").and_then(Value::as_object).is_some()
                || map.get("type").and_then(Value::as_str) == Some("object")
        }
        fn pick_object_branch<'a>(map: &'a Map<String, Value>) -> Option<&'a Value> {
            if let Some(Value::Array(arr)) = map.get("anyOf") {
                for v in arr {
                    if let Value::Object(m) = v {
                        if is_object_like(m) {
                            return Some(v);
                        }
                    }
                }
            }
            if let Some(Value::Array(arr)) = map.get("oneOf") {
                for v in arr {
                    if let Value::Object(m) = v {
                        if is_object_like(m) {
                            return Some(v);
                        }
                    }
                }
            }
            None
        }
        match (value, schema) {
            (Value::Object(obj), Value::Object(smap)) => {
                // If union at this level, try to pick an object-like branch
                if smap.contains_key("anyOf") || smap.contains_key("oneOf") {
                    if let Some(branch) = pick_object_branch(smap) {
                        // Re-run on self with chosen branch by mutating in place
                        let mut tmp = Value::Object(obj.clone());
                        prune_unknown_fields_by_schema(&mut tmp, branch);
                        if let Value::Object(new_obj) = tmp {
                            *obj = new_obj;
                        }
                        return;
                    }
                }
                // Regular object with properties
                if let Some(props) = smap.get("properties").and_then(Value::as_object) {
                    // Remove unknown keys
                    let allowed: std::collections::HashSet<&str> =
                        props.keys().map(|k| k.as_str()).collect();
                    obj.retain(|k, _| allowed.contains(k.as_str()));
                    // Recurse
                    for (k, v) in obj.iter_mut() {
                        if let Some(ps) = props.get(k) {
                            prune_unknown_fields_by_schema(v, ps);
                        }
                    }
                    return;
                }
                // Fallback: nothing to prune
            }
            (Value::Array(arr), Value::Object(smap)) => {
                // Handle union at schema level for arrays by picking items from an object-like branch if present
                if let Some(items_schema) = smap.get("items") {
                    for it in arr.iter_mut() {
                        prune_unknown_fields_by_schema(it, items_schema);
                    }
                    return;
                }
                if let Some(branch) = pick_object_branch(smap) {
                    for it in arr.iter_mut() {
                        prune_unknown_fields_by_schema(it, branch);
                    }
                    return;
                }
            }
            _ => {}
        }
    }

    /// Convenience: prune unknown fields and then deserialize into a typed model.
    pub fn parse_args_with_pruning<T: DeserializeOwned>(
        mut raw: Value,
        schema: &Value,
    ) -> Result<T, String> {
        prune_unknown_fields_by_schema(&mut raw, schema);
        let snapshot = raw.clone();
        let bytes = serde_json::to_vec(&raw).map_err(|e| {
            format!(
                "Could not serialise JSON value for path-aware deserialisation: {}",
                e
            )
        })?;
        let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
        match serde_path_to_error::deserialize::<_, T>(&mut deserializer) {
            Ok(v) => Ok(v),
            Err(e) => {
                let path = e.path().to_string();
                let inner = e.into_inner();
                let msg = format!(
                    "JSON → struct deserialisation failed at {}: {}. Input: {}",
                    path, inner, snapshot
                );
                Err(msg)
            }
        }
    }

    /// Trait implemented for structures that can be exposed to the model as a
    /// callable *tool* (i.e. function).
    ///
    /// The default `as_tool` implementation turns the [`schemars`] schema into
    /// the `parameters` object expected by OpenAI.
    pub trait IntoFunction: JsonSchema {
        /// Name under which the model should call the tool.
        const NAME: &'static str;
        /// Optional human-readable description.
        const DESCRIPTION: &'static str = "";

        /// Convert the Rust type into a [`Function`] definition.
        fn convert_into_function() -> Function {
            // Build full JSON-schema for the type
            let schema = schemars::schema_for!(Self);
            let mut parameters: Value =
                serde_json::to_value(&schema).expect("serialising JSON schema failed");

            // Strip keys that are not wanted by the OpenAI spec and process schema
            if let Value::Object(ref mut map) = parameters {
                map.remove("$schema");
                map.remove("title");
                // Ensure `additionalProperties` is always present (default: false)
                map.entry("additionalProperties")
                    .or_insert_with(|| Value::Bool(false));
            }

            // NOTE: From experimentation, it seems like OpenAI does not like
            // - $ref pointing into the root (e.g., #/$defs/* or #/definitions/*)
            // - unions (anyOf/oneOf) with additionalProperties: false
            // - unions (anyOf/oneOf) with vastly different schemas... -> 500 internal server error
            // Hence we simplify the schema a lot.

            // Inline local refs first so downstream transforms see real structures
            inline_local_refs(&mut parameters);

            // Recursively process the schema to replace oneOf with anyOf and add additionalProperties: false
            process_schema_recursively(&mut parameters);

            // Finally, flatten unions into a single object with optional fields
            flatten_unions(&mut parameters);

            // Collapse unions of const string branches into string enums
            collapse_const_string_unions(&mut parameters);

            // Enforce full required arrays and nullable optional fields
            enforce_full_required_and_nullable(&mut parameters);

            Function {
                name: Self::NAME.to_string(),
                description: if Self::DESCRIPTION.is_empty() {
                    None
                } else {
                    Some(Self::DESCRIPTION.to_string())
                },
                parameters,
                strict: true,
            }
        }
    }

    /// Convenience helper – returns the [`Function`]
    #[inline]
    pub fn tool<T: IntoFunction>() -> Function {
        T::convert_into_function()
    }

    #[cfg(all(test, feature = "schema"))]
    mod tests {
        use super::*;
        use serde_json::json;

        fn process(mut v: Value) -> Value {
            process_schema_recursively(&mut v);
            v
        }

        #[test]
        fn replaces_oneof_with_anyof_and_sets_required_and_additional_properties() {
            let schema = json!({
                "type": "object",
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "a": {"type": "string"},
                            "b": {"type": "integer"}
                        }
                    }
                ]
            });

            let out = process(schema);

            // oneOf removed, anyOf present
            let obj = out.as_object().unwrap();
            assert!(obj.get("oneOf").is_none());
            let any_of = obj.get("anyOf").and_then(|v| v.as_array()).unwrap();
            assert_eq!(any_of.len(), 1);

            // Branch has required all keys and additionalProperties=false
            let branch = any_of[0].as_object().unwrap();
            // additionalProperties should be false on the branch
            assert_eq!(
                branch.get("additionalProperties"),
                Some(&Value::Bool(false))
            );

            let required = branch.get("required").and_then(|v| v.as_array()).unwrap();
            let req_keys: Vec<String> = required
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            // Sorted order guaranteed by implementation
            assert_eq!(req_keys, vec!["a".to_string(), "b".to_string()]);

            // Also the top-level object got additionalProperties=false
            assert_eq!(obj.get("additionalProperties"), Some(&Value::Bool(false)));
        }

        #[test]
        fn sets_additional_properties_false_for_plain_object_and_properties_without_type() {
            // Case 1: explicit object type
            let schema1 = json!({
                "type": "object",
                "properties": {"x": {"type": "number"}}
            });
            let out1 = process(schema1);
            assert_eq!(out1.get("additionalProperties"), Some(&Value::Bool(false)));

            // Case 2: properties present without explicit type
            let schema2 = json!({
                "properties": {"y": {"type": "boolean"}}
            });
            let out2 = process(schema2);
            assert_eq!(out2.get("additionalProperties"), Some(&Value::Bool(false)));
        }

        #[test]
        fn anyof_only_sets_required_for_object_like_branches() {
            let schema = json!({
                "anyOf": [
                    {"type": "string"},
                    {"type": "object", "properties": {"x": {"type": "integer"}, "y": {"type": "string"}}}
                ]
            });
            let out = process(schema);
            let any_of = out.get("anyOf").and_then(|v| v.as_array()).unwrap();

            // First branch (string) should not get required
            assert!(any_of[0].get("required").is_none());

            // Second branch should have required and additionalProperties=false
            let br2 = any_of[1].as_object().unwrap();
            assert_eq!(br2.get("additionalProperties"), Some(&Value::Bool(false)));
            let req = br2.get("required").and_then(|v| v.as_array()).unwrap();
            let keys: Vec<&str> = req.iter().map(|v| v.as_str().unwrap()).collect();
            assert_eq!(keys, vec!["x", "y"]);
        }

        #[test]
        fn recurses_into_nested_structures() {
            let schema = json!({
                "type": "object",
                "properties": {
                    "inner": {
                        "oneOf": [
                            {"type": "object", "properties": {"a": {"type": "number"}}},
                            {"type": "array", "items": {"oneOf": [
                                {"type": "object", "properties": {"z": {"type": "string"}}},
                                {"type": "null"}
                            ]}}
                        ]
                    }
                }
            });

            let out = process(schema);

            // Drill down: properties.inner.anyOf exists
            let inner = out.get("properties").unwrap().get("inner").unwrap();
            let any1 = inner.get("anyOf").and_then(|v| v.as_array()).unwrap();

            // First branch object: has required ["a"] and additionalProperties=false
            let b0 = any1[0].as_object().unwrap();
            assert_eq!(b0.get("additionalProperties"), Some(&Value::Bool(false)));
            let req0 = b0.get("required").and_then(|v| v.as_array()).unwrap();
            assert_eq!(req0.len(), 1);
            assert_eq!(req0[0], Value::String("a".into()));

            // Second branch is array whose items had oneOf -> now anyOf
            let items = any1[1].get("items").unwrap();
            let any2 = items.get("anyOf").and_then(|v| v.as_array()).unwrap();
            // its first branch object gets required ["z"] and additionalProperties=false
            let inner_obj = any2[0].as_object().unwrap();
            assert_eq!(
                inner_obj.get("additionalProperties"),
                Some(&Value::Bool(false))
            );
            let reqz = inner_obj
                .get("required")
                .and_then(|v| v.as_array())
                .unwrap();
            assert_eq!(reqz, &vec![Value::String("z".into())]);
        }

        #[test]
        fn inlines_local_refs_and_removes_defs() {
            let mut schema = json!({
                "$defs": {
                    "Foo": {
                        "type": "object",
                        "properties": {"a": {"type": "string"}}
                    }
                },
                "$ref": "#/$defs/Foo"
            });

            inline_local_refs(&mut schema);

            // Should inline to the Foo object and remove $defs
            let obj = schema.as_object().unwrap();
            assert!(obj.get("$defs").is_none());
            assert!(obj.get("definitions").is_none());
            // Now has properties.a
            let props = obj.get("properties").and_then(|v| v.as_object()).unwrap();
            assert!(props.contains_key("a"));
        }

        #[test]
        fn flattens_unions_merges_properties_and_adds_note() {
            let mut schema = json!({
                "description": "Test union schema",
                "anyOf": [
                    {"type": "object", "properties": {"a": {"type": "string"}}},
                    {"type": "object", "properties": {"b": {"type": "integer"}}}
                ]
            });

            flatten_unions(&mut schema);

            let obj = schema.as_object().unwrap();
            assert_eq!(obj.get("type"), Some(&Value::String("object".into())));
            // required must be removed making fields optional
            assert!(obj.get("required").is_none());
            assert_eq!(obj.get("additionalProperties"), Some(&Value::Bool(false)));
            let props = obj.get("properties").and_then(|v| v.as_object()).unwrap();
            assert!(props.contains_key("a"));
            assert!(props.contains_key("b"));

            // description note appended
            let desc = obj.get("description").and_then(|v| v.as_str()).unwrap();
            assert!(desc.contains("union flattened"));

            // Each property should carry an annotation of variant membership
            let a_desc = props
                .get("a")
                .and_then(|v| v.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let b_desc = props
                .get("b")
                .and_then(|v| v.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            assert!(a_desc.contains("Valid in union variants: 1"));
            assert!(b_desc.contains("Valid in union variants: 2"));
        }

        #[test]
        fn flattens_unions_uses_string_const_discriminant() {
            let mut schema = json!({
                "description": "Union with discriminant",
                "anyOf": [
                    {
                        "type": "object",
                        "properties": {
                            "kind": {"type": "string", "const": "alpha"},
                            "x": {"type": "number"}
                        }
                    },
                    {
                        "type": "object",
                        "properties": {
                            "kind": {"type": "string", "const": "beta"},
                            "y": {"type": "string"}
                        }
                    }
                ]
            });

            flatten_unions(&mut schema);

            let obj = schema.as_object().unwrap();
            assert_eq!(obj.get("type"), Some(&Value::String("object".into())));
            let props = obj.get("properties").and_then(|v| v.as_object()).unwrap();
            // Expect merged properties include kind, x, y
            assert!(props.contains_key("kind"));
            assert!(props.contains_key("x"));
            assert!(props.contains_key("y"));

            // Descriptions should reference the discriminant field name and its values
            let kind_desc = props
                .get("kind")
                .and_then(|v| v.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let x_desc = props
                .get("x")
                .and_then(|v| v.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let y_desc = props
                .get("y")
                .and_then(|v| v.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            assert!(kind_desc.contains("Valid when kind is:"));
            assert!(x_desc.contains("Valid when kind is: alpha"));
            assert!(y_desc.contains("Valid when kind is: beta"));
        }

        #[test]
        fn flattens_union_with_conflicting_property_types_into_anyof() {
            let mut schema = json!({
                "oneOf": [
                    {"type": "object", "properties": {"x": {"type": "string"}}},
                    {"type": "object", "properties": {"x": {"type": "integer"}}}
                ]
            });

            // normalize oneOf -> anyOf, then flatten
            process_schema_recursively(&mut schema);
            flatten_unions(&mut schema);

            let props = schema
                .get("properties")
                .and_then(|v| v.as_object())
                .unwrap();
            let x = props.get("x").unwrap();
            // x should be a schema with anyOf [string, integer]
            let any = x.get("anyOf").and_then(|v| v.as_array()).unwrap();
            assert_eq!(any.len(), 2);
            let types: std::collections::HashSet<&str> = any
                .iter()
                .map(|v| v.get("type").and_then(|t| t.as_str()).unwrap())
                .collect();
            assert!(types.contains("string") && types.contains("integer"));
        }

        #[test]
        fn collapses_nested_string_const_anyof_to_enum() {
            let mut schema = json!({
                "anyOf": [
                    {"anyOf": [
                        {"type": "string", "const": "multiple_choice"},
                        {"type": "string", "const": "fill_in_the_blank"}
                    ]},
                    {"type": "string", "const": "translate"}
                ]
            });

            collapse_const_string_unions(&mut schema);

            let obj = schema.as_object().unwrap();
            assert!(obj.get("anyOf").is_none());
            assert_eq!(obj.get("type"), Some(&Value::String("string".into())));
            let en = obj.get("enum").and_then(|v| v.as_array()).unwrap();
            let vals: Vec<String> = en.iter().map(|v| v.as_str().unwrap().to_string()).collect();
            assert_eq!(
                vals,
                vec![
                    "fill_in_the_blank".to_string(),
                    "multiple_choice".to_string(),
                    "translate".to_string()
                ]
            );
        }

        #[test]
        fn keeps_union_when_non_string_or_mixed() {
            let mut schema = json!({
                "anyOf": [
                    {"type": "string", "const": "a"},
                    {"type": "integer"}
                ]
            });

            collapse_const_string_unions(&mut schema);
            let obj = schema.as_object().unwrap();
            assert!(obj.get("enum").is_none());
            assert!(obj.get("anyOf").is_some());
        }

        #[test]
        fn deduplicates_enum_values_on_collapse() {
            let mut schema = json!({
                "anyOf": [
                    {"type": "string", "const": "x"},
                    {"anyOf": [
                        {"type": "string", "const": "x"},
                        {"type": "string", "const": "y"}
                    ]}
                ]
            });

            collapse_const_string_unions(&mut schema);
            let obj = schema.as_object().unwrap();
            let en = obj.get("enum").and_then(|v| v.as_array()).unwrap();
            let vals: Vec<String> = en.iter().map(|v| v.as_str().unwrap().to_string()).collect();
            assert_eq!(vals, vec!["x".to_string(), "y".to_string()]);
        }

        #[test]
        fn inlines_same_ref_multiple_times() {
            let mut schema = json!({
                "$defs": {
                    "Foo": {
                        "type": "object",
                        "properties": {"a": {"type": "string"}}
                    }
                },
                "type": "object",
                "properties": {
                    "x": {"$ref": "#/$defs/Foo"},
                    "y": {"$ref": "#/$defs/Foo"}
                }
            });

            inline_local_refs(&mut schema);

            let obj = schema.as_object().unwrap();
            // defs should be removed because no local $ref remain
            assert!(obj.get("$defs").is_none());
            assert!(obj.get("definitions").is_none());

            let props = obj.get("properties").and_then(|v| v.as_object()).unwrap();
            let x = props.get("x").and_then(|v| v.as_object()).unwrap();
            let y = props.get("y").and_then(|v| v.as_object()).unwrap();
            assert!(x.get("$ref").is_none());
            assert!(y.get("$ref").is_none());
            assert!(x.get("properties").is_some());
            assert!(y.get("properties").is_some());
        }

        #[test]
        fn handles_recursive_ref_without_infinite_loop() {
            let mut schema = json!({
                "$defs": {
                    "Node": {
                        "type": "object",
                        "properties": {
                            "value": {"type": "integer"},
                            "next": {"$ref": "#/$defs/Node"}
                        }
                    }
                },
                "$ref": "#/$defs/Node"
            });

            inline_local_refs(&mut schema);

            let obj = schema.as_object().unwrap();
            // In recursive case, local $ref remains for next; hence $defs must be kept
            assert!(obj.get("$defs").is_some());
            let props = obj.get("properties").and_then(|v| v.as_object()).unwrap();
            let next = props.get("next").and_then(|v| v.as_object()).unwrap();
            assert_eq!(
                next.get("$ref"),
                Some(&Value::String("#/$defs/Node".to_string()))
            );
        }

        // ---------------- New tests for enforce_full_required_and_nullable ----------------
        #[test]
        fn sets_required_all_properties_and_makes_optional_nullable() {
            let mut schema = json!({
                "type": "object",
                "properties": {
                    "a": {"type": "string"},
                    "b": {"type": "integer"}
                },
                "required": ["a"]
            });

            enforce_full_required_and_nullable(&mut schema);

            let obj = schema.as_object().unwrap();
            // required should contain both a and b
            let req = obj.get("required").and_then(|v| v.as_array()).unwrap();
            let reqs: Vec<String> = req
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            assert_eq!(reqs, vec!["a".to_string(), "b".to_string()]);
            assert_eq!(obj.get("additionalProperties"), Some(&Value::Bool(false)));

            let props = obj.get("properties").and_then(|v| v.as_object()).unwrap();
            let a = props.get("a").unwrap().as_object().unwrap();
            // a was required originally: keep non-nullable
            if let Some(Value::Array(arr)) = a.get("type") {
                assert!(!arr.iter().any(|t| t.as_str() == Some("null")));
            }
        }
    }
}

#[cfg(all(test, feature = "schema"))]
mod tests_prune {
    use super::tool_builder::prune_unknown_fields_by_schema;
    use serde_json::json;

    #[test]
    fn prunes_unknown_fields_simple() {
        let mut value = json!({
            "a": 1,
            "x": "drop"
        });
        let schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "integer"}
            }
        });

        prune_unknown_fields_by_schema(&mut value, &schema);
        assert_eq!(value, json!({"a": 1}));
    }

    #[test]
    fn prunes_unknown_fields_in_array_items() {
        let mut value = json!([
            {"k": "v", "extra": 1},
            {"extra": 2, "k": "w"}
        ]);
        let schema = json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "k": {"type": "string"}
                }
            }
        });

        prune_unknown_fields_by_schema(&mut value, &schema);
        assert_eq!(
            value,
            json!([
                {"k": "v"},
                {"k": "w"}
            ])
        );
    }
}
