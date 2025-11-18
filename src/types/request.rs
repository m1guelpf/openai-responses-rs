use macon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
    InputItem, MessageStatus, Model, ReasoningConfig, Role, ServiceTier, TextConfig, Tool,
    ToolChoice, Truncation,
};

/// The Request object.
#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
pub struct Request {
    /// Model ID used to generate the response.
    /// OpenAI offers a wide range of models with different capabilities, performance characteristics, and price points. Refer to the [model guide](https://platform.openai.com/docs/models) to browse and compare available models.
    pub model: Model,
    /// Text, image, or file inputs to the model, used to generate a response.
    pub input: Input,
    /// Specify additional output data to include in the model response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<Include>>,
    /// Inserts a system (or developer) message as the first item in the model's context.
    /// When using along with `previous_response_id`, the instructions from a previous response will be not be carried over to the next response. This makes it simple to swap out system (or developer) messages in new responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// An upper bound for the number of tokens that can be generated for a response, including visible output tokens and [reasoning tokens](https://platform.openai.com/docs/guides/reasoning).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    /// Set of 16 key-value pairs that can be attached to an object. This can be useful for storing additional information about the object in a structured format, and querying for objects via API or the dashboard.
    /// Keys are strings with a maximum length of 64 characters. Values are strings with a maximum length of 512 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// Whether to allow the model to run tool calls in parallel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// The unique ID of the previous response to the model. Use this to create multi-turn conversations.
    /// Learn more about [conversation state](https://platform.openai.com/docs/guides/conversation-state).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    /// Configuration options for [reasoning models](https://platform.openai.com/docs/guides/reasoning).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningConfig>,
    /// Specifies the latency tier to use for processing the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    /// Whether to store the generated model response for later retrieval via API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    /// If set to true, the model response data will be streamed to the client as it is generated using [server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#event_stream_format).
    ///
    /// This flag is automatically set depending on the function you call, so it's best to not touch it.
    #[doc(hidden)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// What sampling temperature to use, between 0 and 2.
    /// Higher values like 0.8 will make the output more random, while lower values like 0.2 will make it more focused and deterministic.
    /// We generally recommend altering this or `top_p` but not both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Configuration options for a text response from the model. Can be plain text or structured JSON data. Learn more:
    /// - [Text inputs and outputs](https://platform.openai.com/docs/guides/text)
    /// - [Structured Outputs](https://platform.openai.com/docs/guides/structured-outputs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextConfig>,
    /// How the model should select which tool (or tools) to use when generating a response.
    ///
    /// See the `tools` parameter to see how to specify which tools the model can call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// An array of tools the model may call while generating a response. You can specify which tool to use by setting the `tool_choice` parameter.
    /// The two categories of tools you can provide the model are:
    /// - **Built-in tools**: Tools that are provided by OpenAI that extend the model's capabilities, like [web search](https://platform.openai.com/docs/guides/tools-web-search) or [file search](https://platform.openai.com/docs/guides/tools-file-search). Learn more about [built-in tools](https://platform.openai.com/docs/guides/tools).
    /// - **Function calls (custom tools)**: Functions that are defined by you, enabling the model to call your own code. Learn more about [function calling](https://platform.openai.com/docs/guides/function-calling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// An alternative to sampling with temperature, called nucleus sampling, where the model considers the results of the tokens with `top_p` probability mass. So 0.1 means only the tokens comprising the top 10% probability mass are considered.
    /// We generally recommend altering this or `temperature` but not both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// The truncation strategy to use for the model response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<Truncation>,
    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse. [Learn more](https://platform.openai.com/docs/guides/safety-best-practices#end-user-ids).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            user: None,
            text: None,
            tools: None,
            store: None,
            top_p: None,
            stream: None,
            include: None,
            metadata: None,
            reasoning: None,
            truncation: None,
            tool_choice: None,
            temperature: None,
            instructions: None,
            service_tier: None,
            model: Model::GPT4o,
            max_output_tokens: None,
            parallel_tool_calls: None,
            previous_response_id: None,
            input: Input::Text(String::new()),
        }
    }
}

/// Text, image, or file inputs to the model, used to generate a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Input {
    /// A text input to the model, equivalent to a text input with the user role.
    Text(String),
    /// A list of one or many input items to the model, containing different content types.
    List(Vec<InputListItem>),
}

/// One or many input items to the model, containing different content types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputListItem {
    /// A message input to the model with a role indicating instruction following hierarchy.
    /// Instructions given with the `developer` or `system` role take precedence over instructions given with the `user` role.
    /// Messages with the `assistant` role are presumed to have been generated by the model in previous interactions.
    Message(InputMessage),
    /// An internal identifier for an item to reference.
    #[serde(rename = "item_reference")]
    ItemRef {
        /// The ID of the item to reference.
        id: String,
    },
    /// An item representing part of the context for the response to be generated by the model.
    /// Can contain text, images, and audio inputs, as well as previous assistant responses and tool call outputs.
    #[serde(untagged)]
    Item(InputItem),
}

/// A message input to the model with a role indicating instruction following hierarchy.
///
/// Instructions given with the `developer` or `system` role take precedence over instructions given with the `user` role.
/// Messages with the `assistant` role are presumed to have been generated by the model in previous interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMessage {
    /// The role of the message input.
    pub role: Role,
    /// Text, image, or audio input to the model, used to generate a response. Can also contain previous assistant responses.
    pub content: ContentInput,
}

/// Text, image, or audio input to the model, used to generate a response. Can also contain previous assistant responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentInput {
    Text(String),
    List(Vec<ContentItem>),
}

/// A message input to the model with a role indicating instruction following hierarchy, retrieved from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIInputMessage {
    /// The role of the message input.
    pub role: Role,
    /// Text, image, or audio input to the model, used to generate a response. Can also contain previous assistant responses.
    pub content: ContentInput,
    /// The status of the message. Populated when the message is returned via API.
    #[serde(skip_serializing)]
    pub status: Option<MessageStatus>,
}

/// Text, image, or audio input to the model, used to generate a response. Can also contain previous assistant responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentItem {
    /// A text input to the model.
    #[serde(rename = "input_text")]
    Text { text: String },
    /// An image input to the model. Learn about [image inputs](https://platform.openai.com/docs/guides/vision).
    #[serde(rename = "input_image")]
    Image {
        /// The detail level of the image to be sent to the model.
        detail: ImageDetail,
        /// The ID of the file to be sent to the model.
        file_id: Option<String>,
        /// The URL of the image to be sent to the model. A fully qualified URL or base64 encoded image in a data URL.
        image_url: Option<String>,
    },
    /// A file input to the model.
    #[serde(rename = "input_file")]
    File {
        /// The content of the file to be sent to the model.
        file_data: Option<String>,
        /// The ID of the file to be sent to the model.
        file_id: Option<String>,
        /// The name of the file to be sent to the model.
        filename: Option<String>,
    },
}

/// The detail level of the image sent to the model.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageDetail {
    #[default]
    Auto,
    Low,
    High,
    Medium,
}

/// Additional output data to include in the model response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Include {
    /// Include the search results of the file search tool call.
    #[serde(rename = "file_search_call.results")]
    FileSearchResults,
    /// Include image urls from the input message.
    #[serde(rename = "message.input_image.image_url")]
    InputImageURLs,
    /// Include image urls from the computer call output.
    #[serde(rename = "computer_call_output.output.image_url")]
    ComputerCallImageURLs,
    /// Includes an encrypted version of reasoning tokens in reasoning item
    /// outputs. This enables reasoning items to be used in multi-turn
    /// conversations when using the Responses API statelessly (like when the
    /// store parameter is set to false, or when an organization is enrolled in
    /// the zero data retention program)
    #[serde(rename = "reasoning.encrypted_content")]
    ReasoningEncryptedContent,
}
