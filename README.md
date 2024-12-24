sussy_ducky_bot v2

AI telegram bot with convenient features.

### How to run
```
cargo install sqlx-cli --no-default-features --features sqlite
sqlx db create
sqlx mig run
cargo r --release
```
Also don't forget about populating .env

### What the heck is this rewrite
- rewritten from scratch for the third time
- hosted on a server, gtx 1660 super
- custom models (some controvertial, use at own risk)
- conversation continuation using sql caching
- ollama model verification
- ratelimits (soon)

### TODO
- [x] Basic AI bot
- [x] Typing indicator on different thread
- [x] Caption support (no images yet)
- [x] Database integration
- [x] Async
- [x] Ollama model pulling
- [x] Gemini API
- [x] GroqCloud API
- [x] Multiple local model support
- [x] Groq support
- [x] Threads to continue conversations
- [x] Ollama model verification
- [ ] Ratelimits
- [ ] Image support
- [ ] Docker
- [ ] Stats command (requires mapping user id to username)
- [ ] File reorganization
- [ ] OpenAI voice mode replies
