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

2. **Dependencies**: The project uses the following Rust crates:
   - `reqwest` for HTTP requests.
   - `teloxide` for Telegram bot API.
   - `lambda_http` for AWS Lambda integration.
   - `tracing` for logging.

## Commands

- `/help` & `/start` - Displays help text.
- `/caveman` - Uses the "caveman" model.
- `/llama` - Uses the "llama3.1 70b" model.
- `/pixtral` - Uses the "pixtral 12b" vision model.
- `/vision` - Uses the "qwen2-vl 7b" vision model.

## Running the Bot

To run the bot, ensure all environment variables are set, then execute the deploy.sh script

## Contributing

Feel free to open issues or submit pull requests for improvements or bug fixes.

## License

This project is licensed under the MIT License. See `LICENSE` for details.