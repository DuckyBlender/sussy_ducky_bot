# sussy_ducky_bot

## Overview

sussy_ducky_bot is a serverless telegram bot written in Rust that interacts with models from OpenRouter. It handles commands from users and processes messages with various AI models.

> [!NOTE]
> NEW! Now for three months the bot also uses the free flux[schnell] model from together.ai

## Features

- **Serverless**: Made for AWS Lambda.
- **Free**: Hosted on AWS Lambda, uses free OpenRouter models.
- **Image Handling**: Supports vision models.

## Setup

1. **Environment Variables**: Ensure you have the following environment variables set:
   - `TELEGRAM_BOT_TOKEN`: Your Telegram bot token
   - `OPENROUTER_KEY`: API key for OpenRouter
   - `TOGETHER_KEY`: API key for together.ai

2. **Cargo Lambda**: Install the `cargo-lambda` tool. More information can be found [here](https://www.cargo-lambda.info/guide/installation.html).

> [!IMPORTANT]  
> Don't install cargo lambda using `cargo install cargo-lambda`, as it doesn't support ARM64. Because of this, the brew installation is recommended.

3. **AWS CLI**: Ensure you have the AWS CLI installed and configured.

## Commands

- `/help` & `/start` - Displays help text
- `/caveman` - Uses Llama3.1 8b model from OpenRouter with a custom system prompt
- `/llama` - Uses Llama 3.1 70b or 3.2 12b vision model from OpenRouter
- `/lobotomy` - Uses Llama 3.2 1b model from OpenRouter (for fun)
- `/flux` - Uses the free flux[schnell] model from together.ai

## Running the Bot

To run the bot, ensure all environment variables are set, then execute the `deploy.sh` script. It compiles the project in arm64 and deploys it to AWS Lambda.

## Contributing

Feel free to open issues or submit pull requests for improvements or bug fixes.

## License

This project is licensed under the MIT License. See `LICENSE` for details.