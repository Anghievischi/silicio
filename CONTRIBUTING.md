# Contributing to Silício

Thanks for helping! 🇧🇷 Contribuições em português também são bem-vindas: issues, PRs e comentários podem ser escritos em português ou inglês.

## Ways to help

- **Report a bug.** Open a [bug report](https://github.com/Anghievischi/silicio/issues/new?template=bug_report.yml) and attach the output of `silicio --json`. Most hardware bugs can only be fixed with that report.
- **Report missing or wrong hardware data.** A CPU without a codename, a memory vendor shown as a JEDEC code, a GPU with missing sensors: use the [hardware report](https://github.com/Anghievischi/silicio/issues/new?template=hardware_report.yml) template.
- **Suggest a feature** with the [feature request](https://github.com/Anghievischi/silicio/issues/new?template=feature_request.yml) template.
- **Translate the interface.** The UI is in Brazilian Portuguese; English and other languages are on the roadmap. Open an issue first so the string extraction can be planned together.
- **Send a pull request.** Issues labeled `good first issue` are a good start.

## Development setup

You need [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 20+, [pnpm](https://pnpm.io) and, on Linux, the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (`webkit2gtk-4.1`).

```bash
git clone https://github.com/Anghievischi/silicio.git
cd silicio
pnpm install
pnpm tauri dev
```

Useful commands:

| Command | What it does |
|---------|--------------|
| `pnpm tauri dev` | Runs the app with hot reload |
| `pnpm check` | Type-checks the Svelte/TypeScript code |
| `cargo test` (in `src-tauri`) | Runs the Rust unit tests |
| `cargo clippy --all-targets` (in `src-tauri`) | Lints the Rust code |
| `cargo run -- --json` (in `src-tauri`) | Prints the hardware report |

## Guidelines

### Code

- Match the style of the surrounding code. Rust is formatted with `cargo fmt`; comments and user-facing strings are in Brazilian Portuguese for now.
- Keep platform code behind `#[cfg(target_os = "linux")]` / `#[cfg(windows)]` modules, with a fallback for other systems, as in `src-tauri/src/hw/`.
- Hardware decoders need tests. Add a fixture under `src-tauri/tests/fixtures/` when possible, and **remove serial numbers** and other personal identifiers from dumps before committing.
- New data shown in the UI must also appear in `--json`.

### Clean-room rule

Silício is an independent reimplementation. **Do not contribute code, tables, images or text copied or derived from CPU-Z or any other proprietary tool**, including decompiled code. Use public documentation (Intel/AMD manuals, JEDEC standards, the Linux kernel, `pci.ids`) and cite the source in a comment.

### Commits and pull requests

- One topic per pull request. Describe what changed, why, and how you tested it, ideally with the hardware you used.
- Make sure `pnpm check`, `cargo test` and `cargo clippy --all-targets` pass. CI runs them on Linux and Windows.
- Screenshots are welcome for UI changes.

## Releases

Maintainers publish a release by pushing a tag:

```bash
git tag v0.2.0 && git push origin v0.2.0
```

The [release workflow](.github/workflows/release.yml) builds the Windows and Linux packages and attaches them to the release. Notable changes go to [CHANGELOG.md](CHANGELOG.md).

## Code of Conduct

By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
