# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- English interface. The language follows the system and can be switched live at the bottom of the sidebar; the choice is remembered.

### Changed

- The backend returns codes instead of Portuguese text for cache types, instruction groups, chassis types and SPD voltage, so the interface can translate them. This changes the `--json` output for those fields.

## [0.1.1]

### Fixed

- **Windows: the app did not open.** Reading CPU sensors initialized COM in multithreaded mode on the main thread before the window was created, so the window's `OleInitialize` failed with `RPC_E_CHANGED_MODE` and the app closed silently. Hardware commands now run on worker threads and sensors are created on first use.

### Added

- Crash log: fatal errors are written to `silicio-crash.log` in the system temp folder (`%TEMP%` on Windows, `/tmp` on Linux).
- Smoke test that opens the real window and visits every tab; it runs on Windows in CI and caught the bug above.

## [0.1.0]

First public version.

### Added

- **Processor** tab: name, codename, microarchitecture, process, signature, microcode, caches, instructions, topology, live clock and load per thread, temperature.
- **Motherboard** tab: system, board, BIOS, chipset, southbridge and PCIe link of each graphics adapter.
- **Memory** tab: type, channels, speed, installed modules, live usage and diagnostics (single channel, XMP disabled, mixed kits).
- **SPD** tab: DDR4 decoder with manufacturer, part number, manufacturing date, organization, minimum timings and JEDEC timings table, verified against `decode-dimms`.
- **Graphics** tab: AMD and Intel through Linux sysfs, NVIDIA through NVML; clocks, load, temperature, power and VRAM.
- Root helper (`silicio-helper`) started with `pkexec` for socket, core voltage and real bus clock on Linux.
- Windows support through WMI for BIOS, memory modules and graphics adapters.
- **Benchmark** tab: single-thread and multi-thread scores, multi/single ratio, history of the last 10 runs, and a stress test with live clock and temperature charts that warns about sustained throttling.
- `--json` report, `--bench` scores, light and dark theme, Ctrl+1…6 shortcuts.
- Installers for Windows (`.exe`, `.msi`, portable) and Linux (`.deb`, `.rpm`, `.AppImage`).
- Community files: contributing guide, code of conduct, security policy, issue and pull request templates, CI on Linux and Windows.

[Unreleased]: https://github.com/Anghievischi/silicio/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/Anghievischi/silicio/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Anghievischi/silicio/releases/tag/v0.1.0
