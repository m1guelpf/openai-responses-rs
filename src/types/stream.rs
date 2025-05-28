use serde::{Deserialize, Serialize};

use super::{Annotation, OutputContent, OutputItem, Response, SummaryContent};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    /// An event that is emitted when a response is created.
    #[serde(rename = "response.created")]
    ResponseCreated {
        /// The response that was created.
        response: Response,
    },
    /// Emitted when the response is in progress.
    #[serde(rename = "response.in_progress")]
    ResponseInProgress {
        /// The response that is in progress.
        response: Response,
    },
    /// Emitted when the model response is complete.
    #[serde(rename = "response.completed")]
    ResponseCompleted {
        /// Properties of the completed response.
        response: Response,
    },
    /// An event that is emitted when a response fails.
    #[serde(rename = "response.failed")]
    ResponseFailed {
        /// The response that failed.
        response: Response,
    }, // todo: probably we just care about error?
    /// An event that is emitted when a response finishes as incomplete.
    #[serde(rename = "response.incomplete")]
    ResponseIncomplete {
        /// The response that was incomplete.
        response: Response,
    },
    /// Emitted when a new output item is added.
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded {
        /// The output item that was added.
        item: OutputItem,
        /// The index of the output item that was added.
        output_index: u64,
    },
    /// Emitted when an output item is marked done.
    #[serde(rename = "response.output_item.done")]
    OutputItemDone {
        /// The output item that was marked done.
        item: OutputItem,
        /// The index of the output item that was marked done.
        output_index: u64,
    },
    /// Emitted when a new content part is added.
    #[serde(rename = "response.content_part.added")]
    ContentPartAdded {
        /// The index of the content part that was added.
        content_index: u64,
        /// The ID of the output item that the content part was added to.
        item_id: String,
        /// The index of the output item that the content part was added to.
        output_index: u64,
        /// The content part that was added.
        part: OutputContent,
    },
    /// Emitted when a content part is done.
    #[serde(rename = "response.content_part.done")]
    ContentPartDone {
        /// The index of the content part that is done.
        content_index: u64,
        /// The ID of the output item that the content part was added to.
        item_id: String,
        /// The index of the output item that the content part was added to.
        output_index: u64,
        /// The content part that is done.
        part: OutputContent,
    },
    /// Emitted when there is an additional text delta.
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta {
        /// The index of the content part that the text delta was added to.
        content_index: u64,
        /// The text delta that was added.
        delta: String,
        /// The ID of the output item that the text delta was added to.
        item_id: String,
        /// The index of the output item that the text delta was added to.
        output_index: u64,
    },
    /// Emitted when a text annotation is added.
    #[serde(rename = "response.output_text.annotation.added")]
    OutputTextAnnotationAdded {
        /// The annotation that was added.
        annotation: Annotation,
        /// The index of the annotation that was added.
        annotation_index: u64,
        /// The index of the content part that the text annotation was added to.
        content_index: u64,
        /// The ID of the output item that the text annotation was added to.
        item_id: String,
        /// The index of the output item that the text annotation was added to.
        output_index: u64,
    },
    /// Emitted when text content is finalized.
    #[serde(rename = "response.output_text.done")]
    OutputTextDone {
        /// The index of the content part that the text content is finalized.
        content_index: u64,
        /// The ID of the output item that the text content is finalized.
        item_id: String,
        /// The index of the output item that the text content is finalized.
        output_index: u64,
        /// The text content that is finalized.
        text: String,
    },
    /// Emitted when a new reasoning summary part is added.
    #[serde(rename = "response.reasoning_summary_part.added")]
    ReasoningSummaryPartAdded {
        /// The ID of the item this summary part is associated with.
        item_id: String,
        /// The index of the output item this summary part is associated with.
        output_index: u64,
        /// The index of the summary part within the reasoning summary.
        summary_index: u64,
        /// The summary part that was added.
        part: SummaryContent,
    },
    /// Emitted when a reasoning summary part is completed.
    #[serde(rename = "response.reasoning_summary_part.done")]
    ReasoningSummaryPartDone {
        /// The ID of the item this summary part is associated with.
        item_id: String,
        /// The index of the output item this summary part is associated with.
        output_index: u64,
        /// The index of the summary part within the reasoning summary.
        summary_index: u64,
        /// The summary part that was added.
        part: SummaryContent,
    },
    /// Emitted when there is an additional text delta.
    #[serde(rename = "response.reasoning_summary_text.delta")]
    ReasoningSummaryTextDelta {
        /// The text delta that was added to the summary.
        delta: String,
        /// The ID of the item this summary text delta is associated with.
        item_id: String,
        /// The index of the output item this summary text delta is associated with.
        output_index: u64,
        /// The index of the summary part within the reasoning summary.
        summary_index: u64,
    },
    /// Emitted when text content is finalized.
    #[serde(rename = "response.reasoning_summary_text.done")]
    ReasoningSummaryTextDone {
        /// The ID of the item this summary text is associated with.
        item_id: String,
        /// The index of the output item this summary text is associated with.
        output_index: u64,
        /// The index of the summary part within the reasoning summary.
        summary_index: u64,
        /// The full text of the completed reasoning summary.
        text: String,
    },
    /// Emitted when there is a partial refusal text.
    #[serde(rename = "response.refusal.delta")]
    RefusalDelta {
        /// The index of the content part that the refusal text is added to.
        content_index: u64,
        /// The refusal text that is added.
        delta: String,
        /// The ID of the output item that the refusal text is added to.
        item_id: String,
        /// The index of the output item that the refusal text is added to.
        output_index: u64,
    },
    /// Emitted when refusal text is finalized.
    #[serde(rename = "response.refusal.done")]
    RefusalDone {
        /// The index of the content part that the refusal text is finalized.
        content_index: u64,
        /// The ID of the output item that the refusal text is finalized.
        item_id: String,
        /// The index of the output item that the refusal text is finalized.
        output_index: u64,
        /// The refusal text that is finalized.
        refusal: String,
    },
    /// Emitted when there is a partial function-call arguments delta.
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta {
        /// The function-call arguments delta that is added.
        delta: String,
        /// The ID of the output item that the function-call arguments delta is added to.
        item_id: String,
        /// The index of the output item that the function-call arguments delta is added to.
        output_index: u64,
    },
    /// Emitted when function-call arguments are finalized.
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone {
        /// The function-call arguments.
        arguments: String,
        /// The ID of the item.
        item_id: String,
        /// The index of the output item.
        output_index: u64,
    },
    /// Emitted when a file search call is initiated.
    #[serde(rename = "response.file_search_call.in_progress")]
    FileSearchCallInitiated {
        /// The ID of the output item that the file search call is initiated.
        item_id: String,
        /// The index of the output item that the file search call is initiated.
        output_index: u64,
    },
    /// Emitted when a file search is currently searching.
    #[serde(rename = "response.file_search_call.searching")]
    FileSearchCallSearching {
        /// The ID of the output item that the file search call is searching.
        item_id: String,
        /// The index of the output item that the file search call is searching.
        output_index: u64,
    },
    /// Emitted when a file search call is completed (results found).
    #[serde(rename = "response.file_search_call.completed")]
    FileSearchCallCompleted {
        /// The ID of the output item that the file search call completed at.
        item_id: String,
        /// The index of the output item that the file search call completed at.
        output_index: u64,
    },
    /// Emitted when a web search call is initiated.
    #[serde(rename = "response.web_search_call.in_progress")]
    WebSearchCallInitiated {
        /// Unique ID for the output item associated with the web search call.
        item_id: String,
        /// The index of the output item that the web search call is associated with.
        output_index: u64,
    },
    /// Emitted when a web search call is executing.
    #[serde(rename = "response.web_search_call.searching")]
    WebSearchCallSearching {
        /// Unique ID for the output item associated with the web search call.
        item_id: String,
        /// The index of the output item that the web search call is associated with.
        output_index: u64,
    },
    /// Emitted when a web search call is completed.
    #[serde(rename = "response.web_search_call.completed")]
    WebSearchCallCompleted {
        /// Unique ID for the output item associated with the web search call.
        item_id: String,
        /// The index of the output item that the web search call is associated with.
        output_index: u64,
    },
    /// Emitted when an error occurs.
    #[serde(rename = "error")]
    Error {
        /// The error code.
        code: Option<String>,
        /// The error message.
        message: String,
        /// The error parameter.
        param: Option<String>,
    },
}
