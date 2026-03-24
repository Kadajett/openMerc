# Contributing to OpenMerc

## Development Setup
1. **Clone the repository**
   ```bash
   git clone https://github.com/your-org/openmerc.git
   cd openmerc
   ```
2. **Install Rust toolchain** (stable) and required tools:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup component add rustfmt clippy
   ```
3. **Install pre‑commit hooks** (optional but recommended):
   ```bash
   cargo install pre-commit
   pre-commit install
   ```
4. **Run the test suite** to ensure everything works:
   ```bash
   cargo test
   ```

## Workflow
- **Branch naming**: `feature/<short-description>` or `bugfix/<short-description>`.
- **Commit messages**: Follow the conventional commits style (`feat:`, `fix:`, `docs:` etc.).
- **Pull Requests**: Target the `main` branch, include a clear description, and reference any related issue numbers.
- **CI**: CI runs `cargo fmt`, `cargo clippy`, and `cargo test` on every PR.

## Code Style
- Use `rustfmt` formatting (`cargo fmt`).
- Lint with `cargo clippy` and address warnings.
- Keep public APIs stable; add tests for new functionality.

## Adding New Features
1. Write the implementation in `src/`.
2. Add unit tests in `tests/`.
3. Update documentation in `README.md` or relevant module docs.
4. Ensure the new code passes `cargo test` and `cargo clippy`.

## Reporting Issues
- Use the GitHub Issues board.
- Include steps to reproduce, expected vs actual behavior, and any relevant logs.

Thank you for contributing! 🎉
