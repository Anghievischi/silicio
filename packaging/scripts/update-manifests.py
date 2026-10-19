#!/usr/bin/env python3
"""Atualiza os manifestos de Scoop, winget e AUR para uma versão publicada.

Baixa os arquivos da release no GitHub, calcula o SHA-256 de cada um e reescreve:

- bucket/silicio.json                       (Scoop: `scoop bucket add silicio <repo>`)
- packaging/winget/<versão>/*.yaml          (enviar com `wingetcreate submit`)
- packaging/aur/PKGBUILD e .SRCINFO         (publicar no AUR como silicio-bin)

Uso: python3 packaging/scripts/update-manifests.py 0.1.2
"""

import hashlib
import json
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

REPO = "Anghievischi/silicio"
ROOT = Path(__file__).resolve().parents[2]
DESCRIPTION = "Modern open-source CPU-Z alternative: CPU, motherboard, memory, SPD and GPU with live data"


def sha256(url: str) -> str:
    print(f"baixando {url}")
    digest = hashlib.sha256()
    with urllib.request.urlopen(url) as response:
        while chunk := response.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def scoop(version: str, portable_hash: str) -> None:
    base = f"https://github.com/{REPO}/releases/download"
    manifest = {
        "version": version,
        "description": DESCRIPTION,
        "homepage": f"https://github.com/{REPO}",
        "license": "MIT",
        "architecture": {
            "64bit": {
                "url": f"{base}/v{version}/Silicio-portable.exe#/Silicio.exe",
                "hash": portable_hash,
            }
        },
        "bin": "Silicio.exe",
        "shortcuts": [["Silicio.exe", "Silício"]],
        "checkver": "github",
        "autoupdate": {
            "architecture": {"64bit": {"url": f"{base}/v$version/Silicio-portable.exe#/Silicio.exe"}}
        },
    }
    path = ROOT / "bucket" / "silicio.json"
    path.parent.mkdir(exist_ok=True)
    path.write_text(json.dumps(manifest, indent=4, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"escrito {path}")


def winget(version: str, setup_hash: str) -> None:
    ident = "Anghievischi.Silicio"
    folder = ROOT / "packaging" / "winget" / version
    if folder.parent.exists():
        shutil.rmtree(folder.parent)
    folder.mkdir(parents=True)
    header = "# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kind}.1.9.0.schema.json\n"
    files = {
        f"{ident}.yaml": (
            "version",
            f"PackageIdentifier: {ident}\nPackageVersion: {version}\nDefaultLocale: en-US\n"
            "ManifestType: version\nManifestVersion: 1.9.0\n",
        ),
        f"{ident}.installer.yaml": (
            "installer",
            f"PackageIdentifier: {ident}\nPackageVersion: {version}\nInstallerType: nullsoft\n"
            "Scope: user\nInstallModes:\n- interactive\n- silent\nUpgradeBehavior: install\n"
            "Installers:\n- Architecture: x64\n"
            f"  InstallerUrl: https://github.com/{REPO}/releases/download/v{version}/Silicio_{version}_x64-setup.exe\n"
            f"  InstallerSha256: {setup_hash.upper()}\n"
            "ManifestType: installer\nManifestVersion: 1.9.0\n",
        ),
        f"{ident}.locale.en-US.yaml": (
            "defaultLocale",
            f"PackageIdentifier: {ident}\nPackageVersion: {version}\nPackageLocale: en-US\n"
            "Publisher: Diógenes A. Anghievischi\n"
            f"PublisherUrl: https://github.com/Anghievischi\nPackageName: Silício\n"
            f"PackageUrl: https://github.com/{REPO}\nLicense: MIT\n"
            f"LicenseUrl: https://github.com/{REPO}/blob/main/LICENSE\n"
            f"ShortDescription: {DESCRIPTION}.\n"
            "Tags:\n- cpu\n- cpu-z\n- hardware\n- system-information\n- benchmark\n"
            f"ReleaseNotesUrl: https://github.com/{REPO}/releases/tag/v{version}\n"
            "ManifestType: defaultLocale\nManifestVersion: 1.9.0\n",
        ),
    }
    for name, (kind, body) in files.items():
        (folder / name).write_text(header.format(kind=kind) + body, encoding="utf-8")
    print(f"escrito {folder}")


def aur(version: str, deb_hash: str) -> None:
    folder = ROOT / "packaging" / "aur"
    folder.mkdir(parents=True, exist_ok=True)
    pkgbuild = f"""# Maintainer: Diógenes A. Anghievischi <32557254+Anghievischi@users.noreply.github.com>
pkgname=silicio-bin
pkgver={version}
pkgrel=1
pkgdesc="{DESCRIPTION}"
arch=('x86_64')
url="https://github.com/{REPO}"
license=('MIT')
depends=('webkit2gtk-4.1' 'gtk3' 'polkit')
optdepends=('i2c-tools: SPD tab on systems where the driver is not loaded automatically')
provides=('silicio')
conflicts=('silicio')
source=("Silicio_${{pkgver}}_amd64.deb::$url/releases/download/v$pkgver/Silicio_${{pkgver}}_amd64.deb"
        "LICENSE::https://raw.githubusercontent.com/{REPO}/v$pkgver/LICENSE")
sha256sums=('{deb_hash}'
            'SKIP')

package() {{
  bsdtar -xf data.tar.gz -C "$pkgdir"
  # O pacote .deb traz uma cópia do helper em /usr/bin; o app usa a de /usr/lib/silicio.
  rm -f "$pkgdir/usr/bin/silicio-helper"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}}
"""
    (folder / "PKGBUILD").write_text(pkgbuild, encoding="utf-8")
    if shutil.which("makepkg"):
        srcinfo = subprocess.run(["makepkg", "--printsrcinfo"], cwd=folder, capture_output=True, text=True, check=True)
        (folder / ".SRCINFO").write_text(srcinfo.stdout, encoding="utf-8")
    print(f"escrito {folder}")


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    version = sys.argv[1].lstrip("v")
    base = f"https://github.com/{REPO}/releases/download/v{version}"
    scoop(version, sha256(f"{base}/Silicio-portable.exe"))
    winget(version, sha256(f"{base}/Silicio_{version}_x64-setup.exe"))
    aur(version, sha256(f"{base}/Silicio_{version}_amd64.deb"))


if __name__ == "__main__":
    main()
