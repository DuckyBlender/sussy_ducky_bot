# Sussy Ducky Bot (because all the good names were taken)

## Description

This is a AI telegram bot which currently supports Mistral. It requires Ollama to run in the background. This bot is mainly for fun and learning purposes. It is probably not very useful for anything else.

## Features
- Supports variations of Mistral (and two custom prompt models)
- Supports Tinyllama
- Supports models from Perplexity AI
- Context using replies (this currently only works with one message - the one the user is replying to)
- Other small gimmicks

## Todo

- [ ] MarkdownV2 support

## Prerequisites

- At least $1 on perplexity.ai
- Ollama
- Rust

## Installation

1. Install Ollama following the instructions on its [official website](https://ollama.ai/).
2. Download the following models: 

bash
```
ollama pull dolphin-mistral
ollama pull mistral-openorca
ollama pull tinyllama
```

3. Clone this repository: `git clone https://github.com/DuckyBlender/sussy_ducky_bot`
4. Navigate to the cloned repository: `cd sussy_ducky_bot`
5. Install the caveman and racist model model:

bash
```
ollama create caveman-mistral -f ./custom_models/caveman/Modelfile
ollama create racist-mistral -f ./custom_models/racist/Modelfile
```

## Usage

1. Set the .env from the .env.example
2. Make sure ollama is running in the background
3. Run the bot with `cargo run --release`

# Docker
(keep in mind im a total docker noob so there may be a better way to do this. this is also untested)
`docker run -e TELOXIDE_TOKEN=xxx OPENAI_KEY=xxx BOT_NAME=xxx `

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

## License

[MIT](https://choosealicense.com/licenses/mit/)
