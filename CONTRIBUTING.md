# Contributing to Silício

Thanks for helping! 🇧🇷 Contribuições em português também são bem-vindas: issues, PRs e comentários podem ser escritos em português ou inglês.

## Ways to help

- **Report a bug.** Open a [bug report](https://github.com/Anghievischi/silicio/issues/new?template=bug_report.yml) and attach the output of `silicio --json`. Most hardware bugs can only be fixed with that report.
- **Report missing or wrong hardware data.** A CPU without a codename, a memory vendor shown as a JEDEC code, a GPU with missing sensors: use the [hardware report](https://github.com/Anghievischi/silicio/issues/new?template=hardware_report.yml) template.
- **Suggest a feature** with the [feature request](https://github.com/Anghievischi/silicio/issues/new?template=feature_request.yml) template.
- **Translate the interface.** Copy `src/lib/i18n/en.ts` to a new file (for example `es.ts`), translate the texts, and register it in `src/lib/i18n/index.svelte.ts` (`LOCALES` and `dictionaries`). TypeScript checks that no key is missing.
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
| `cargo run -- --session 300` (in `src-tauri`) | Records a 300 s gaming session and prints the report |
| `SILICIO_SMOKE=1 cargo test --test smoke` (in `src-tauri`, after `pnpm build`) | Opens the real window, visits every tab and checks the app exits cleanly (always runs on Windows CI) |

## Guidelines

### Code

- Match the style of the surrounding code. Rust is formatted with `cargo fmt`; code comments are in Brazilian Portuguese. User-facing text never goes in components: add it to `src/lib/i18n/pt-BR.ts` and `en.ts`, and have the backend return codes (like `"notebook"`) that the interface translates.
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

Maintainers publish a release with one command, after adding the changes to the `[Unreleased]` section of [CHANGELOG.md](CHANGELOG.md):

```bash
packaging/scripts/release.sh 0.5.0
```

The script bumps the version in `package.json`, `Cargo.toml`/`Cargo.lock` and `tauri.conf.json`, turns `[Unreleased]` into the new version, opens the "Versão X" pull request, waits for CI and merges it, then pushes the `vX.Y.Z` tag.

The tag starts the [release workflow](.github/workflows/release.yml), which builds the Windows and Linux packages, signs them for the auto-updater (secret `TAURI_SIGNING_PRIVATE_KEY`) and attaches them, plus `latest.json`, to the release.

When the release is out, the script runs `packaging/scripts/update-manifests.py`, which rewrites the Scoop manifest (`bucket/`), the winget manifests (`packaging/winget/`) and the AUR `PKGBUILD`/`.SRCINFO` (`packaging/aur/`), and commits them to `main`. Two steps stay manual because they publish outside GitHub:

- winget: `wingetcreate submit packaging/winget/<version>`
- AUR: copy `PKGBUILD` and `.SRCINFO` to the `silicio-bin` AUR repository and push.

## Code of Conduct

By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
