# Sussy Ducky Bot (because all the good names were taken)

## Description

This is a AI telegram bot which currently supports Mistral. It requires Ollama to run in the background. This bot is mainly for fun and learning purposes. It is probably not very useful for anything else.

## Features
- Supports variations of Mistral
- Supports LLaVa (image recognition)
- Supports Tinyllama
- Supports models from Perplexity AI
- Other small gimmicks

## Todo

- [x] Mistral (base model)
- [x] Llava (image recognition)
- [x] Bot rewrite (to make it more modular)
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
ollama pull llava
ollama pull dolphin-mistral
ollama pull mistral-openorca
ollama pull tinyllama
```

3. Clone this repository: `git clone https://github.com/DuckyBlender/sussy_ducky_bot`
4. Navigate to the cloned repository: `cd sussy_ducky_bot`
5. Install the caveman model:

bash
```
ollama create caveman-mistral -f ./caveman/Modelfile
```

## Usage

1. Set the .env from the .env.example
2. Make sure ollama is running in the background
3. Run the bot with `cargo run --release`

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

## License

[MIT](https://choosealicense.com/licenses/mit/)
