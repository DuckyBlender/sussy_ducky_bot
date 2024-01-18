# Sussy Ducky Bot (because all the good names were taken)

## Description

This is a AI telegram bot which currently supports Mistral. It requires Ollama to run in the background.

## Features
- Supports Mistral and some of it's variants
- Supports llava (image recognition)
- Threaded (can handle multiple requests at once)
- Other small gimmicks

## Todo

- [x] Mistral (base model)
- [x] Llava (image recognition)
- [x] Threaded
- [ ] MarkdownV2 support

## Prerequisites

- Ollama
- Rust

## Installation

1. Install Ollama following the instructions on its [official website](https://ollama.ai/).
2. Download the following models: `mistral`, `llava`, `dolphin-mistral`, `mistral-openorca`

bash
```
ollama pull mistral
ollama pull llava
ollama pull dolphin-mistral
ollama pull mistral-openorca
```

2. Clone this repository: `git clone https://github.com/DuckyBlender/sussy_ducky_bot`
3. Navigate to the cloned repository: `cd sussy_ducky_bot`

## Usage

1. Set the .env from the .env.example
1. Start Ollama in the background.
2. Run the bot with `cargo run --release`

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

## License

[MIT](https://choosealicense.com/licenses/mit/)