# openmerc

A fast, parallel LLM CLI tool powered by diffusion models.

## Installation

```bash
# Build and install
./install.sh
```

## Usage

- **No arguments** – launch the interactive TUI.
- `--headless "PROMPT"` – run in headless mode and output the result.
- `--init` – create a default `.openmerc.toml` configuration file.
- `--session NAME` – resume a saved session.
- `--version` – show version information.
- `-h, --help` – display help.

## Examples

```bash
openmerc                     # start UI
openmerc --headless "Explain quantum computing"
openmerc --init
openmerc --session work
```

## License

MIT © Jeremy Stover
