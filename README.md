# Telegram Bot with OpenAI Integration

## Overview

Serverless Rust Telegram bot that interacts with OpenAI and other AI models through GroqCloud and OpenRouter. It handles commands from users and processes messages with various AI models.

## Features

- **Serverless**: Made for AWS Lambda.
- **Free**: Hosted on AWS Lambda, uses free groqcloud and openrouter APIs.
- **Image Handling**: Supports vision models.

## Setup

1. **Environment Variables**: Ensure you have the following environment variables set:
   - `TELEGRAM_BOT_TOKEN`: Your Telegram bot token.
   - `GROQ_KEY`: API key for GroqCloud.
   - `OPENROUTER_KEY`: API key for OpenRouter.

2. **Cargo Lambda**: Install the `cargo-lambda` tool. More information can be found [here](https://www.cargo-lambda.info/guide/installation.html).

> [!IMPORTANT]  
> Don't install cargo lambda using `cargo install cargo-lambda`, as it doesn't support ARM64. Because of this, the brew installation is recommended.

3. **AWS CLI**: Ensure you have the AWS CLI installed and configured.

## Commands

- `/help` & `/start` - Displays help text.
- `/caveman` - Uses the "llama3.1 70b" model from GroqCloud with a custom system prompt.
- `/llama` - Uses the "llama3.1 70b" model from GroqCloud
- `/pixtral` - Uses the "pixtral 12b" vision model from OpenRouter.
- `/vision` - Uses the "qwen2-vl 7b" uncensored vision model from OpenRouter.

## Running the Bot

To run the bot, ensure all environment variables are set, then execute the `deploy.sh` script. It compiles the project in arm64 and deploys it to AWS Lambda.

## Contributing

Feel free to open issues or submit pull requests for improvements or bug fixes.

## License

This project is licensed under the MIT License. See `LICENSE` for details.