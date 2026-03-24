# openMerc

![CI Status](https://github.com/yourorg/yourrepo/actions/workflows/ci.yml/badge.svg)
![Merc Review](https://github.com/yourorg/yourrepo/actions/workflows/merc-review.yml/badge.svg)

## Description
OpenMerc is a diffusion‑powered LLM terminal UI. It runs a TUI by default, supports headless prompt mode, and can resume saved sessions.

## Install
```bash
# Build from source
cargo build --release
# Or install via script
./install.sh
```

## Usage
```bash
# Launch interactive UI (default)
openmerc

# Headless mode
openmerc --headless "Explain quantum computing"

# Initialise default config
openmerc --init

# Resume a session
openmerc --session "my‑project"
```

## CLI Flags
- `--headless PROMPT` – run in headless mode with the given prompt.
- `--init` – create a default `.openmerc.toml` in the current directory.
- `--session NAME` – resume a previously saved session.
- `--version` – show version.
- `--help` – show help.

## Architecture
```
+-------------------+          +-------------------+
|   UI (TUI)       |  <--->   |   Mercury Client |
+-------------------+          +-------------------+
          |                               |
          v                               v
+-------------------+          +-------------------+
|   Honcho Context |  <--->   |   Diffusion Engine |
+-------------------+          +-------------------+
```

## Credits
- Stefano Ermon, Aditya Grover, Volodymyr Kuleshov, and contributors.
