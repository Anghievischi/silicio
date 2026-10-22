# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.6.2]

### Removed

- **The PawnIO driver and the CPU temperature on Windows.** Installing PawnIO from the 0.6.1 installer crashed Windows (blue screen) on a user's PC. Silício no longer installs, offers or uses any kernel driver on Windows. Without one, Windows has no reliable way to read the processor sensor, so the CPU temperature, core voltage, real bus clock and the heat warning are now Linux-only. If you installed PawnIO through 0.6.1, you can remove it in Settings → Apps.

### Changed

- On Windows the Processor tab shows the socket and external clock right away, without the Unlock button or a UAC prompt.

## [0.6.1]

### Fixed

- The Windows installer failed to build with the bundled PawnIO setup, so 0.6.0 shipped only for Linux. This version brings all of 0.6.0 to Windows.

## [0.6.0]

### Added

- **Storage tab**: every drive with model, capacity, interface (NVMe with its PCIe link, SATA with the link speed, USB), temperature and the free space of each partition. The health comes from the drive's own SMART data: life left on SSDs, total written, power-on hours, unsafe shutdowns, read/write errors and bad sectors, summed up as Good, Caution or Bad. On Linux it is read through udisks2 without root (LUKS and LVM partitions are traced back to their drive); on Windows through the storage driver without administrator.
- **Battery tab** (laptops only): health as the current capacity against the design capacity, charge cycles, charge level, power draw, estimated time left or to full, voltage and chemistry.
- Storage and battery cards on the Overview and sections in the exported report.
- **The Windows installer offers to install the PawnIO driver**, so CPU temperature and max clock work without downloading anything else. The official PawnIO installer is bundled unmodified and asks for UAC itself; the question is skipped when the driver is already installed, and automatic updates do not ask again after a "No".

### Changed

- With 11 tabs, the tab numbers hide in medium-sized windows so the bar fits on one line; Ctrl+0 opens the tenth tab.
- The Windows installer and uninstaller (`setup.exe`) use the Silício icon instead of the generic NSIS one.

## [0.5.0]

### Added

- **DDR5 modules** in the SPD tab: size, organization, manufacturer, part number, minimum timings and the JEDEC timings table (Linux, through the `spd5118` driver).
- **XMP and AMD EXPO profiles**: speed, voltage and CL, tRCD, tRP, tRAS, tRC and tRFC1 of each profile (XMP 2.0 on DDR4, XMP 3.0 with the profile names and EXPO on DDR5), in the SPD tab and in the exported report.
- **Windows: CPU temperature, max clock, core voltage and real bus clock** through the [PawnIO](https://pawnio.eu) driver (AMD Zen and Intel). Click **Unlock** in the Processor tab and allow the UAC prompt; a small read-only helper keeps sending the temperature while the app is open, and the app asks again on the next start. The max clock on AMD comes from CPPC, the same way Linux's `amd-pstate` computes it.
- Socket, voltage and external clock from SMBIOS on Windows.
- **Windows: battery level of Bluetooth headsets, mice and keyboards** (the same value shown in Windows Settings) and **camera formats** with every resolution and maximum frame rate, read through Media Foundation without turning the camera on.
- Max clock on Linux when cpufreq does not report it, and AMD Zen 5 (family 1Ah) in the privileged readings.
- **Heat warning**: if the processor stays at 95 °C or more for 30 seconds, a banner explains what to check (fans, dust, thermal paste). It can be snoozed for 30 minutes.

### Changed

- On Windows the SPD tab explains that module details are in the Memory tab instead of showing Linux instructions.

### Fixed

- Windows: after updating over an older version, shortcuts and the taskbar kept showing the old icon. The installer now refreshes the Windows icon cache.

## [0.4.0]

### Added

- **Upgrades** tab (Ctrl+9): suggestions computed from the detected hardware.
  - **Graphics card**: whether a new card fits (desktop PCIe x16 slot, external GPU over Thunderbolt/USB4, or none on laptops without them), the slot's PCIe generation, a range matched to the CPU so it does not bottleneck, current examples with their lane count, the recommended power supply, a warning about x8 cards on PCIe 3.0 and Intel Arc left out on CPUs without Resizable BAR.
  - **Memory**: add a matching module for dual channel, fill free slots or replace the kit, respecting the board's maximum, type, form factor and speed; soldered memory is recognized.
  - **Processor**: socket, the fastest CPUs it accepts, end-of-life platforms and the BIOS check; soldered (BGA) CPUs on laptops and mini PCs.
  - Also shown on the Overview (best next step), Motherboard and Memory tabs, in the exported report and in `--json`.

### Changed

- New retro-futuristic look: square panels, condensed display type for titles and numbers, monospace text, block meters, a top bar with numbered tabs and a status bar. Fonts are bundled, so it works offline.
- New icon.
- The shareable card and the exported HTML report follow the new look. The card embeds the app's fonts, so the PNG looks the same everywhere.
- New screenshots in the README.
- On Windows the window has its own title bar, with minimize, maximize and close buttons in the app's style.

## [0.3.0]

### Added

- **Peripherals** tab:
  - **Displays** decoded from EDID: manufacturer, model (or the panel part number on laptops), size, native resolution, refresh range, color depth, interface, HDR and BT.2020 support, manufacture date, plus the monitors in use with their scaling.
  - **USB and Bluetooth devices** with their function (keyboard, mouse, camera, audio, controller, receiver…), USB generation and speed, max power and driver. Gaming mice that expose a macro keyboard are shown as mice.
  - **Peripheral batteries** (mice, keyboards, controllers, headsets) on Linux.
  - **Audio** inputs and outputs with the card or codec, connection, default device (Windows) and the format in use.
  - **Cameras** with every format, resolution and maximum frame rate on Linux (read through V4L2 without turning the camera on), names on Windows.
- **Peripheral tests**: keyboard (every key, physical layout, n-key rollover), mouse polling rate, buttons and wheel, left/right speakers and a frequency sweep, full-screen dead pixel check and a camera preview with its real resolution and frame rate.
- **Shareable specs card**: a 1200×630 PNG (dark or light) with CPU, GPU, memory, motherboard, display and the last benchmark score, saved to Downloads or copied to the clipboard.
- The exported report now includes the peripherals.

## [0.2.0]

### Added

- **Overview** tab, now the start screen: CPU, system and chassis at a glance, animated gauges for CPU load, average clock, temperature, memory in use and GPU load, plus summary cards that link to each tab.
- **Live core map** on the Overview and Processor tabs: cores grouped like on the chip (each group shares an L3 cache, the CCX on AMD), colored by load, with the clock of each core and a bar per thread. Intel hybrid processors show P and E cores (Linux through `cpu_core`/`cpu_atom`, Windows through `GetLogicalProcessorInformationEx`, which also gives the real core topology on Windows now).
- **Report export** (sidebar or Ctrl+E): everything Silício collected, organized by component, saved to the Downloads folder as an HTML page (readable, printable to PDF, follows light/dark theme), Markdown for forums, or raw JSON. "Copy as text" puts the Markdown on the clipboard. Serial numbers and the computer name are not included.
- App version at the bottom of the sidebar, linking to the release notes; operating system and kernel in the report.
- Loading placeholders on the Overview instead of plain text.

### Changed

- Release workflow: the release is created first as a pre-release, the packages are uploaded to it, and only then it becomes the latest release, so the direct download links never point to a release without files.

## [0.1.3]

### Added

- **Windows:** chipset and southbridge names, PCIe link of each device, graphics cards through DXGI (correct VRAM above 4 GB), GPU load and VRAM usage for every vendor through Windows performance counters, CPU microcode, max clock (Intel) and ACPI thermal zone temperature when no CPU sensor is available (marked as such).
- Built-in subset of the PCI ID database, used on Windows and on Linux systems without `pci.ids`.
- Auto-update: the app checks for new signed releases and installs them (Windows installer, `.deb`, `.rpm`, AppImage).
- Scoop bucket (`scoop bucket add silicio https://github.com/Anghievischi/silicio`), winget manifests and AUR `PKGBUILD` (`silicio-bin`), generated by `packaging/scripts/update-manifests.py`.

## [0.1.2]

### Added

- English interface. The language follows the system and can be switched live at the bottom of the sidebar; the choice is remembered.

### Fixed

- SPD voltage showed floating-point noise (`1.2000000476837158 V`).

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

[Unreleased]: https://github.com/Anghievischi/silicio/compare/v0.6.2...HEAD
[0.6.2]: https://github.com/Anghievischi/silicio/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/Anghievischi/silicio/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/Anghievischi/silicio/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Anghievischi/silicio/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Anghievischi/silicio/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Anghievischi/silicio/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Anghievischi/silicio/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/Anghievischi/silicio/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Anghievischi/silicio/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Anghievischi/silicio/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Anghievischi/silicio/releases/tag/v0.1.0
