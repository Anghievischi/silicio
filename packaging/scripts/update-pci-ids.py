#!/usr/bin/env python3
"""Gera src-tauri/data/pci-subset.ids a partir da base pci.ids do sistema.

O Windows não tem a base pci.ids, então o Silício embute um subconjunto com os
fabricantes de chipset, GPU, rede e armazenamento mais comuns. No Linux a base
do sistema continua tendo prioridade.

Uso: python3 packaging/scripts/update-pci-ids.py [/usr/share/hwdata/pci.ids]

A pci.ids é mantida pelo PCI ID Project (https://pci-ids.ucw.cz/) e
distribuída sob GPL-2.0-or-later ou BSD-3-Clause; o cabeçalho original, com a
licença, é preservado no arquivo gerado.
"""

import sys
from pathlib import Path

VENDORS = {
    "1002", "1022", "8086", "10de",                  # AMD/ATI, AMD, Intel, NVIDIA
    "1b21", "1106", "1912", "1b4b", "1d6a",          # ASMedia, VIA, Renesas, Marvell, Aquantia
    "10ec", "14e4", "168c", "17cb", "c0a9",          # Realtek, Broadcom, Qualcomm Atheros, Qualcomm, Micron/Crucial
    "144d", "1987", "1c5c", "15b7", "1e0f", "126f",  # Samsung, Phison, SK hynix, SanDisk/WD, KIOXIA, Silicon Motion
    "1e95", "1344", "1cc1", "2646",                  # SSSTC, Micron, ADATA, Kingston
    "1414",                                          # Microsoft (Hyper-V)
}

source = Path(sys.argv[1] if len(sys.argv) > 1 else "/usr/share/hwdata/pci.ids")
target = Path(__file__).resolve().parents[2] / "src-tauri" / "data" / "pci-subset.ids"

out, header, vendor = [], True, None
for line in source.read_text(encoding="utf-8", errors="replace").splitlines(keepends=True):
    if line.startswith("C "):
        break  # a lista de classes não é usada
    if line.startswith("#"):
        if header:
            out.append(line)
        continue
    if not line.strip():
        continue
    header = False
    if not line.startswith("\t"):
        vendor = line[:4].lower()
        if vendor in VENDORS:
            out.append(line)
    elif not line.startswith("\t\t") and vendor in VENDORS:
        out.append(line)  # dispositivos; subsistemas ficam de fora

target.write_text("".join(out), encoding="utf-8")
print(f"{target}: {len(out)} linhas")
