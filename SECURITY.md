# Security Policy

## Supported versions

Only the [latest release](https://github.com/Anghievischi/silicio/releases/latest) receives security fixes.

## Reporting a vulnerability

Please **do not open a public issue** for security problems. Report them privately through [GitHub Security Advisories](https://github.com/Anghievischi/silicio/security/advisories/new).

Include what you found, how to reproduce it and the affected version and platform. You should get a first reply within 7 days.

## Scope

The most sensitive part of Silício is **`silicio-helper`**, the small program that runs as root on Linux through `pkexec` to read CPU registers (MSR) and the raw SMBIOS table. By design it:

- accepts no arguments and reads no environment-controlled paths;
- only reads fixed files under `/dev/cpu/*/msr` and `/sys/firmware/dmi/tables/`;
- writes nothing except JSON to standard output;
- is installed at `/usr/lib/silicio/silicio-helper` with the polkit action `br.com.silicio.helper` (`packaging/linux/br.com.silicio.helper.policy`).

Anything that lets the helper write to hardware or files, read other paths, or be started without authentication is a vulnerability we want to hear about.

Release binaries are built by GitHub Actions from the tagged source. They are not code-signed yet.
