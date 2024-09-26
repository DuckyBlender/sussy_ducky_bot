cargo clippy --fix --allow-dirty
cargo fmt
cargo lambda build --release --arm64
cargo lambda deploy --env-file .env