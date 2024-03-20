# Sussy Ducky Bot (because all the good names were taken)

## Description

This is a AI telegram bot which currently supports Mistral. It requires Ollama to run in the background. This bot is mainly for fun and learning purposes. It is probably not very useful for anything else.

If you want to use the bot you can add it to your group by clicking [here](https://t.me/sussy_ducky_bot). If you want you can give it permission to delete messages.

## Features

- Supports variations of Mistral (and three custom prompt models)
- Supports Tinyllama
- Supports models from Perplexity AI
- Context using replies (this currently only works with one message - the one the user is replying to)
- Threaded responses (the bot can accept messages while it's still processing the previous one)
- Other small gimmicks

## Todo

- [ ] MarkdownV2 support
- [ ] Refactor bot so it's more modular and more readable (this code is terrible)

## Prerequisites

- At least $1 on perplexity.ai
- Ollama (so at least 8GB of RAM)
- Rust

## Installation

1. Clone this repository: `git clone https://github.com/DuckyBlender/sussy_ducky_bot`
2. Navigate to the cloned repository: `cd sussy_ducky_bot`
3. Install the caveman and racist model model:
4. Install Ollama following the instructions on its [official website](https://ollama.ai/).

## Usage

1. Set the .env from the .env.example
2. Make sure ollama is running in the background
3. Run the bot with `cargo run --release -- --download` (--download to download all of the models)

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

## License

[MIT](https://choosealicense.com/licenses/mit/)
