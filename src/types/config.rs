use macon::Builder;
use serde::{Deserialize, Serialize};

/// The truncation strategy to use for the model response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Truncation {
    /// If the context of this response and previous ones exceeds the model's context window size, the model will truncate the response to fit the context window by dropping input items in the middle of the conversation.
    Auto,
    /// If a model response will exceed the context window size for a model, the request will fail with a 400 error.
    #[default]
    Disabled,
}

/// Configuration options for a text response from the model. Can be plain text or structured JSON data.
///
/// Learn more:
/// - [Text inputs and outputs](https://platform.openai.com/docs/guides/text)
/// - [Structured Outputs](https://platform.openai.com/docs/guides/structured-outputs)
#[derive(Debug, Clone, Builder, Default, Serialize, Deserialize)]
pub struct TextConfig {
    /// An object specifying the format that the model must output.
    pub format: TextFormat,
}

/// An object specifying the format that the model must output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextFormat {
    /// Used to generate text responses.
    #[default]
    Text,
    /// JSON Schema response format. Used to generate structured JSON responses. Learn more about [Structured Outputs](https://platform.openai.com/docs/guides/structured-outputs).
    JsonSchema {
        /// The schema for the response format, described as a JSON Schema object. Learn how to build JSON schemas [here](https://json-schema.org/).
        schema: serde_json::Value,
        /// A description of what the response format is for, used by the model to determine how to respond in the format.
        description: String,
        /// The name of the response format. Must be a-z, A-Z, 0-9, or contain underscores and dashes, with a maximum length of 64.
        name: String,
        /// Whether to enable strict schema adherence when generating the output.
        /// If set to true, the model will always follow the exact schema defined in the schema field. Only a subset of JSON Schema is supported when strict is true.
        /// To learn more, read the [Structured Outputs guide](https://platform.openai.com/docs/guides/structured-outputs).
        strict: Option<bool>,
    },
    /// JSON object response format. An older method of generating JSON responses.
    /// Using `JsonSchema` is recommended for models that support it.
    /// Note that the model will not generate JSON without a system or user message instructing it to do so.
    JsonObject,
}

/// Configuration options for [reasoning models](https://platform.openai.com/docs/guides/reasoning).
/// Only available for o-series models.
#[derive(Debug, Clone, Default, Builder, Serialize, Deserialize)]
pub struct ReasoningConfig {
    /// Constrains effort on reasoning for [reasoning models](https://platform.openai.com/docs/guides/reasoning). Reducing reasoning effort can result in faster responses and fewer tokens used on reasoning in a response.
    pub effort: Option<ReasoningEffort>,
    /// A summary of the reasoning performed by the model. This can be useful for debugging and understanding the model's reasoning process.
    #[serde(rename = "summary")]
    pub generate_summary: Option<SummaryConfig>,
}

/// Constrains effort on reasoning for [reasoning models](https://platform.openai.com/docs/guides/reasoning).
///
/// Reducing reasoning effort can result in faster responses and fewer tokens used on reasoning in a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    High,
    Medium,
}

/// A summary of the reasoning performed by the model. This can be useful for debugging and understanding the model's reasoning process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SummaryConfig {
    Concise,
    Detailed,
}

/// The latency tier to use for processing the request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTier {
    #[default]
    Auto,
    Flex,
    Default,
}
