pub mod config;
mod helpers;
pub mod item;
pub mod request;
pub mod response;
#[cfg(feature = "stream")]
pub mod stream;
pub mod tools;

pub use config::*;
pub use item::*;
pub use request::*;
pub use response::*;
#[cfg(feature = "stream")]
pub use stream::*;
pub use tools::*;

/// The model to use for generating a response.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Model {
    o1,
    #[serde(rename = "o1-mini")]
    o1Mini,
    #[serde(rename = "o1-pro")]
    o1Pro,
    #[serde(rename = "o3-mini")]
    o3Mini,
    #[serde(rename = "gpt-4.5-preview")]
    GPT4_5Preview,
    #[serde(rename = "gpt-4o")]
    GPT4o,
    #[serde(rename = "gpt-4o-mini")]
    GPT4oMini,
    #[serde(rename = "gpt-4o-turbo")]
    GPT4Turbo,
    #[serde(rename = "gpt-4")]
    GPT4,
    #[serde(rename = "gpt-3.5-turbo")]
    GPT3_5Turbo,
    #[serde(rename = "computer-use-preview")]
    ComputerUsePreview,
    #[serde(untagged)]
    Other(String),
}

/// The role of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    System,
    Assistant,
    Developer,
}
