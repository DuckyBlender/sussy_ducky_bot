sussy_ducky_bot v2

### How to run
`cargo install sqlx-cli --no-default-features --features sqlite`
`sqlx db create`
`sqlx mig run`
`cargo r`

### What the heck is this rewrite
- rewritten from scratch
- hosted on a server, gtx 1660 super
- custom models (racist phi)
- conversation continuation using sql caching
- (later) openai voice mode replies
- ratelimits
- paid tier(?)

### bugs
- [ ] fix message length limit

### Order of implementation
- [x] Basic AI bot
- [x] Captions
- [x] Database integration
- [x] Async
- [ ] Ratelimits
- [x] Multiple local model support
- [ ] Message editing progress
- [ ] Groq support
- [ ] Image support
- [x] Threads to continue conversations
- [ ] Docker
