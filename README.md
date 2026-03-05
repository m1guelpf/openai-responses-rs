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
    input: Input::Text("Are semicolons optional in JavaScript?".to_string()),
    instructions: Some("You are a coding assistant that talks like a pirate".to_string()),
    ..Default::default()
}).await?;

println!("{}", response.output_text());
```

To stream the response as it is generated, use the `stream` method:

```rust ignore
use openai_responses::{Client, Request};

// You can also build the `Request` struct with a fluent interface
let mut stream = Client::from_env()?.stream(
    Request::builder()
        .model("gpt-4o")
        .input("Are semicolons optional in JavaScript?")
        .instructions("You are a coding assistant that talks like a pirate")
        .build()
);

while let Some(event) = stream.next().await {
    dbg!(event?);
}
```

---

## Custom headers (Organization & Project)

If you use an OpenAI organization or want to scope usage to a specific project you can instruct the SDK to send `OpenAI-Organization` and `OpenAI-Project` headers:

```rust ignore
use openai_responses::Client;

let client = Client::builder()
    .api_key("sk-my-api-key")
    .organization("org_123")
    .project("my_awesome_project")
    .build()?;

// All requests sent with `client` now include the extra headers.
```

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
