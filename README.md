# Sussy Ducky Bot (because all the good names were taken)

## Description

WIP rewrite branch because the master branch is terrible

## Todo

Add all of the commands:

- [ ] `/perplexity` - Llama 3 with internet access
- [ ] `/claude` - Claude 3 Haiku multimodal
- [ ] `/noviews` - Get a random youtube video with no views
- [ ] `/summarize` - Summarize a youtube video

## Prerequisites

- At least $1 on perplexity.ai
- AWS CLI with credentials set and Amazon text & Cohere text model access in the `us-west-2` region
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
