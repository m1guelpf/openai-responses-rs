# OpenAI Responses SDK

[![crates.io](https://img.shields.io/crates/v/openai_responses.svg)](https://crates.io/crates/openai_responses)
[![download count badge](https://img.shields.io/crates/d/openai_responses.svg)](https://crates.io/crates/openai_responses)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/openai_responses)

An unofficial Rust SDK for the [OpenAI Responses API](https://platform.openai.com/docs/api-reference/responses).

## Usage

To get started, create a new `Client` and call the `create` method with a `Request` object. The `Request` object contains the parameters for the API call, such as the model, instructions, and input. The `create` method returns a `Response` object, which contains the output of the API call.

```rust ignore
use openai_responses::{Client, Request, types::{Input, Model}};

let response = Client::from_env()?.create(Request {
  model: Model::GPT4o,
  instructions: Some("You are a coding assistant that talks like a pirate".to_string()),
  input: Input::Text("Are semicolons optional in JavaScript?".to_string()),
  ..Default::default()
}).await?;

println!("{}", response.output_text());
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
