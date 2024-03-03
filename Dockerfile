# Use an official Rust runtime as a parent image
FROM rust:1.76

# Set the working directory in the container to /usr/src/app
WORKDIR /usr/src/app

# Install Ollama
RUN curl -sSL https://get.ollama.ai | sh

# Download the necessary models
RUN ollama pull dolphin-mistral
RUN ollama pull mistral-openorca
RUN ollama pull tinyllama

# Clone the repository
RUN git clone https://github.com/DuckyBlender/sussy_ducky_bot .

# Navigate to the cloned repository
WORKDIR /usr/src/app/sussy_ducky_bot

# Install the custom models
RUN ollama create caveman-mistral -f ./custom_models/caveman/Modelfile
RUN ollama create racist-mistral -f ./custom_models/racist/Modelfile

# Build the project
RUN cargo build --release

# At the end, set the command to run your binary
CMD ["cargo", "run", "--release"]