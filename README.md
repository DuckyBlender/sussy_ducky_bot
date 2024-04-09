# Sussy Ducky Bot (because all the good names were taken)

## Description

This is a Telegram bot written in Rust. It requires Ollama to run in the background. This bot is mainly for fun and learning purposes. It is probably not very useful for anything else.

If you want to use the bot you can add it to your group by clicking [here](https://t.me/sussy_ducky_bot). If you want you can give it permission to delete messages.

## Features

The bot supports the following commands:

- `/solar`: Generate text using the 10.7B solar LLM. This is the best general-purpose model in this bot.
- `/mistral` or `/m`: Generate text using 7B uncensored dolphin-mistral LLM.
- `/caveman` or `/cv`: Generate text using 7B dolphin-mistral LLM in caveman language [CUSTOM PROMPT MODEL].
- `/racist`: Generate racist responses using 7B dolphin-mistral LLM [CUSTOM PROMPT MODEL].
- `/lobotomy`: Generate nonsense text using 300MB qwen:0.5b-chat-v1.5-q2_K LLM.
- `/tinyllama`: Generate text using 1.1B 8Q tinyllama-openorca LLM.
- `/help`: Show available commands.
- `/ping`: Check the bot's latency.
- `/httpcat`: Get an image of a cat for a given HTTP status code.
- `/noviews`: Get a random video with no views (or very few views).
- `/mixtral`: Generate text using the mixtral-8x7b-instruct model from groq.com.
- `/gemma`: Generate text using the gemma-7b-it model from groq.com.
- `/chatlgbt` or `/lgbt`: Goofy ahh bot which responds with earlier user inputs.
- `/online`: Generate text using the pplx-7b-online model from PerplexityAI [DEV ONLY].

## Todo

- [x] Code refactor
- [x] Streaming support
- [x] Auto-delete invalid commands after 5 seconds (requires permission)
- [x] MarkdownV2 support
- [ ] Maybe even make it serverless?

## Prerequisites

- At least $1 on perplexity.ai
- Ollama (at least 8GB of ram because of the high parameter count models)
- groq.com API key
- AMD or NVIDIA GPU is recommended
- Rust

## Installation

1. Clone this repository: `git clone https://github.com/DuckyBlender/sussy_ducky_bot`
2. Navigate to the cloned repository: `cd sussy_ducky_bot`
3. Install the caveman and racist model model:
4. Install Ollama following the instructions on its [official website](https://ollama.ai/).

## Usage

1. Set the .env from the .env.example
2. Make sure ollama is running in the background
3. Run the bot with `cargo run --release -- --download` (--download to automatically download all of the models)

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

## License

[MIT](https://choosealicense.com/licenses/mit/)
