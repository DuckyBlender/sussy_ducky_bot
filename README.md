# Sussy Ducky Bot (because all the good names were taken)

## Description

This is a Telegram bot written in Rust. It requires Ollama to run in the background. This bot is mainly for fun and learning purposes. It is probably not very useful for anything else.

If you want to use the bot you can add it to your group by clicking [here](https://t.me/sussy_ducky_bot). If you want you can give it permission to delete messages.

## Features

Look at the `Commands` struct in `main.rs` to see all of the commands.
This bot also has experimental image generation support in a custom crate (`comfyui-rs`). This is not very stable. In fact it is very unstable.

## Todo

- [x] Code refactor
- [ ] Major rewrite
- [x] Streaming support
- [x] Auto-delete invalid commands after 5 seconds (requires permission)
- [x] /clone using AWS Bedrock
    - [ ] Generate closest aspect ratio with /clone
- [ ] /emojify
- [x] /jsonify
- [ ] Rate limiting to finally make some paid (but cheap) models public
- [ ] Queue system (so the user knows the bot isn't stuck)
- [ ] MarkdownV2 support
- [ ] Maybe even make it serverless?

## Prerequisites

- At least $1 on perplexity.ai
- Ollama (at least 8GB of ram because of the high parameter count models)
- groq.com API key
- AWS CLI with credentials set and Amazon text & Cohere text model access in the `us-west-2` region
- AMD or NVIDIA GPU is recommended
- ComfyUI in the background with the SDXLTurbo model
- Rust

## Installation

1. Install dependencies: `sudo apt install pkg-config libssl-dev`
2. Clone this repository: `git clone https://github.com/DuckyBlender/sussy_ducky_bot`
3. Navigate to the cloned repository: `cd sussy_ducky_bot`
4. Install the caveman and racist model model:
5. Install Ollama following the instructions on its [official website](https://ollama.ai/).

## Usage

1. Set the .env from the .env.example
2. Make sure ollama is running in the background
3. Run the bot with `cargo run --release -- --download` (--download to automatically download all of the models)

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

## License

[MIT](https://choosealicense.com/licenses/mit/)
