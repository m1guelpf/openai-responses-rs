use serde::{Deserialize, Serialize, de::Visitor, ser::SerializeStruct};
use std::collections::HashMap;

/// A tool the model may call while generating a response.
///
/// The two categories of tools you can provide the model are:
/// - **Built-in tools**: Tools that are provided by OpenAI that extend the model's capabilities, like [web search](https://platform.openai.com/docs/guides/tools-web-search) or [file search](https://platform.openai.com/docs/guides/tools-file-search). Learn more about [built-in tools](https://platform.openai.com/docs/guides/tools).
/// - **Function calls (custom tools)**: Functions that are defined by you, enabling the model to call your own code. Learn more about [function calling](https://platform.openai.com/docs/guides/function-calling).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Tool {
    /// Defines a function in your own code the model can choose to call. Learn more about [function calling](https://platform.openai.com/docs/guides/function-calling).
    Function {
        /// The name of the function to call.
        name: String,
        /// A JSON schema object describing the parameters of the function.
        parameters: serde_json::Value,
        /// Whether to enforce strict parameter validation.
        strict: bool,
        /// A description of the function. Used by the model to determine whether or not to call the function.
        description: Option<String>,
    },
    /// A tool that searches for relevant content from uploaded files. Learn more about the [file search tool](https://platform.openai.com/docs/guides/tools-file-search).
    FileSearch {
        /// The IDs of the vector stores to search.
        vector_store_ids: Vec<String>,
        /// A filter to apply based on file attributes.
        filters: FileSearchFilters,
        /// The maximum number of results to return. This number should be between 1 and 50 inclusive.
        max_num_results: u8,
        /// Ranking options for search.
        ranking_options: RankingOptions,
    },
    #[serde(rename = "computer_use_preview")]
    ComputerUse {
        /// The height of the computer display.
        display_height: u64,
        /// The width of the computer display.
        display_width: u64,
        /// The type of computer environment to control.
        environment: Environment,
    },
    #[serde(rename = "web_search_preview")]
    WebSearch {
        /// High level guidance for the amount of context window space to use for the search.
        search_context_size: SearchContextSize,
        /// Approximate location parameters for the search.
        user_location: Option<UserLocation>,
    },
}

/// Approximate location parameters for the search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserLocation {
    /// The type of location approximation
    pub r#type: UserLocationType,
    /// Free text input for the city of the user, e.g. `San Francisco`.
    pub city: Option<String>,
    /// The two-letter [ISO country code](https://en.wikipedia.org/wiki/ISO_3166-1) of the user, e.g. `US`.
    pub country: Option<String>,
    /// Free text input for the region of the user, e.g. `California`.
    pub region: Option<String>,
    /// The [IANA timezone](https://timeapi.io/documentation/iana-timezones) of the user, e.g. `America/Los_Angeles`.
    pub timezone: Option<String>,
}

/// The type of location approximation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserLocationType {
    #[default]
    Approximate,
}

/// High level guidance for the amount of context window space to use for the search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchContextSize {
    Low,
    High,
    #[default]
    Medium,
}

/// The type of computer environment to control.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Mac,
    Ubuntu,
    Browser,
    Windows,
}

/// A filter to apply based on file attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileSearchFilters {
    /// A filter used to compare a specified attribute key to a given value using a defined comparison operation.
    Single(ComparisonFilter),
    /// Combine multiple filters using and or or.
    Compound(CompoundFilter),
}

/// A filter used to compare a specified attribute key to a given value using a defined comparison operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonFilter {
    /// The key to compare against the value.
    pub key: String,
    /// Specifies the comparison operator.
    pub r#type: ComparisonFilterType,
    /// The value to compare against the attribute key.
    pub value: ComparisonFilterValue,
}

/// The value to compare against the attribute key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComparisonFilterValue {
    Number(f64),
    Boolean(bool),
    String(String),
}

/// Specifies the comparison operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonFilterType {
    #[serde(rename = "eq")]
    Equals,
    #[serde(rename = "ne")]
    NotEqual,
    #[serde(rename = "gt")]
    GreaterThan,
    #[serde(rename = "gte")]
    GreaterThanOrEqual,
    #[serde(rename = "lt")]
    LessThan,
    #[serde(rename = "lte")]
    LessThanOrEqual,
}

/// Combine multiple filters using and or or.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundFilter {
    /// Array of filters to combine.
    pub filters: Vec<FileSearchFilters>,
    /// Type of operation.
    pub r#type: CompoundFilterType,
}

/// Type of operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompoundFilterType {
    And,
    Or,
}

/// Ranking options for search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingOptions {
    /// The ranker to use for the file search.
    pub ranker: String,
    /// The score threshold for the file search, a number between 0 and 1. Numbers closer to 1 will attempt to return only the most relevant results, but may return fewer results.
    pub score_threshold: f32,
}

/// How the model should select which tool (or tools) to use when generating a response.
///
/// See the `tools` parameter to see how to specify which tools the model can call.
#[derive(Debug, Clone, Default)]
pub enum ToolChoice {
    /// The model will not call any tool and instead generates a message.
    None,
    /// The model can pick between generating a message or calling one or more tools.
    #[default]
    Auto,
    /// The model must call one or more tools.
    Required,
    /// Search the contents of uploaded files when generating a response.
    FileSearch,
    /// Include data from the internet in model response generation.
    WebSearchPreview,
    /// Create agentic workflows that enable a model to control a computer interface.
    ComputerUsePreview,
    /// Enable the model to call custom code that you define, giving it access to additional data and capabilities.
    Function(String),
}

impl<'de> Deserialize<'de> for ToolChoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ToolChoiceVisitor;

        impl<'de> Visitor<'de> for ToolChoiceVisitor {
            type Value = ToolChoice;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("string or struct")
            }

            fn visit_str<E>(self, value: &str) -> Result<ToolChoice, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "none" => Ok(ToolChoice::None),
                    "auto" => Ok(ToolChoice::Auto),
                    "required" => Ok(ToolChoice::Required),
                    _ => Err(serde::de::Error::unknown_variant(
                        value,
                        &["none", "auto", "required"],
                    )),
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut record = HashMap::<String, String>::new();

                while let Some((key, value)) = map.next_entry()? {
                    record.insert(key, value);
                }

                let Some(r#type) = record.get("type") else {
                    return Err(serde::de::Error::missing_field("type"));
                };

                match r#type.as_str() {
                    "file_search" => Ok(ToolChoice::FileSearch),
                    "web_search_preview" => Ok(ToolChoice::WebSearchPreview),
                    "computer_use_preview" => Ok(ToolChoice::ComputerUsePreview),
                    "function" => {
                        let Some(name) = record.get("name") else {
                            return Err(serde::de::Error::missing_field("name"));
                        };
                        Ok(ToolChoice::Function(name.clone()))
                    }
                    _ => Err(serde::de::Error::unknown_variant(
                        r#type.as_str(),
                        &[
                            "file_search",
                            "web_search_preview",
                            "computer_use_preview",
                            "function",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_any(ToolChoiceVisitor {})
    }
}

impl Serialize for ToolChoice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::None => serializer.serialize_str("none"),
            Self::Auto => serializer.serialize_str("auto"),
            Self::Required => serializer.serialize_str("required"),
            Self::FileSearch => {
                let mut fn_struct = serializer.serialize_struct("Function", 1)?;
                fn_struct.serialize_field("type", "file_search")?;
                fn_struct.end()
            }
            Self::WebSearchPreview => {
                let mut fn_struct = serializer.serialize_struct("Function", 1)?;
                fn_struct.serialize_field("type", "web_search_preview")?;
                fn_struct.end()
            }
            Self::ComputerUsePreview => {
                let mut fn_struct = serializer.serialize_struct("Function", 1)?;
                fn_struct.serialize_field("type", "computer_use_preview")?;
                fn_struct.end()
            }
            Self::Function(name) => {
                let mut fn_struct = serializer.serialize_struct("Function", 2)?;
                fn_struct.serialize_field("name", name)?;
                fn_struct.serialize_field("type", "function")?;
                fn_struct.end()
            }
        }
    }
}
