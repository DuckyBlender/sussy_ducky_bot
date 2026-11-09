# sussy_ducky_bot

A tiny Telegram bot that chats through a local LLM (llama.cpp's `llama-server`, OpenAI-compatible API).

- **Where it answers:** every message in private chats; in groups, `/ask`, @mentions and replies to its messages.
- **Reply chains:** reply to any message in a conversation to continue it; the whole chain is sent as context (kept in memory, lost on restart).
- **Images:** sent at the highest resolution Telegram has (photos, or images sent as files) when the model has vision (llama-server started with `--mmproj`). This is checked live via `/props`.
- **Formatting:** the model's Markdown is converted to Telegram HTML (bold, italics, code blocks, links, lists, quotes). If Telegram rejects a message, it is resent as plain text.
- **Startup:** messages that arrived while the bot was offline are skipped, not answered.
- **Errors:** the bot reacts with 🤯 instead of posting an error. 🤷 means there was nothing to answer (for example a picture while vision is off).

## System prompts

The default system prompt is "Keep replies short." Users build a shared library of named prompts and each picks one for their own messages:

| Command | What it does |
| --- | --- |
| `/prompts` | list all prompts, marking yours |
| `/prompt [name]` | show a prompt (yours if no name) |
| `/newprompt <name> <text>` | create, or edit one you own; reply to a message with `/newprompt <name>` to use its text |
| `/useprompt <name>` / `/useprompt default` | pick a prompt for yourself / go back to the default |
| `/delprompt <name>` | delete a prompt you created (its users fall back to the default) |

Prompts are stored in `data/prompts.json`.

## Run

```bash
cp .env.example .env   # set TELEGRAM_BOT_TOKEN
mkdir -p data
docker compose up -d --build
docker compose logs -f
```

Logs go to stdout and to `data/logs/sussy_ducky_bot.YYYY-MM-DD.log`. The compose file expects `llama-server` on the Docker host at port 8081.
