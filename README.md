# Silício

Informações do seu hardware com visual moderno: uma alternativa ao CPU-Z para
Linux e Windows, com dados ao vivo e integrações.

Feito com **Tauri 2** (backend em Rust) e **Svelte 5** (interface).

![Ícone do Silício](packaging/icon/silicio.png)

## Download

| Sistema | Download direto |
|---------|-----------------|
| **Windows 10/11** — instalador | [⬇ Silicio-setup.exe](https://github.com/Anghievischi/silicio/releases/latest/download/Silicio-setup.exe) |
| **Windows 10/11** — portátil, sem instalar | [⬇ Silicio-portable.exe](https://github.com/Anghievischi/silicio/releases/latest/download/Silicio-portable.exe) |
| **Windows** — MSI (empresas) | [⬇ Silicio.msi](https://github.com/Anghievischi/silicio/releases/latest/download/Silicio.msi) |
| **Ubuntu, Debian, Mint, Pop!_OS** | [⬇ Silicio-amd64.deb](https://github.com/Anghievischi/silicio/releases/latest/download/Silicio-amd64.deb) |
| **Fedora, openSUSE** | [⬇ Silicio-x86_64.rpm](https://github.com/Anghievischi/silicio/releases/latest/download/Silicio-x86_64.rpm) |
| **Qualquer Linux** — AppImage | [⬇ Silicio-x86_64.AppImage](https://github.com/Anghievischi/silicio/releases/latest/download/Silicio-x86_64.AppImage) |

Os links sempre apontam para a versão mais recente. Versões anteriores estão
na [página de releases](https://github.com/Anghievischi/silicio/releases).

## Instalação

### Windows 10 e 11

Baixe o **[Silicio-setup.exe](https://github.com/Anghievischi/silicio/releases/latest/download/Silicio-setup.exe)**, abra e siga os passos.
Se preferir não instalar, use o **[Silicio-portable.exe](https://github.com/Anghievischi/silicio/releases/latest/download/Silicio-portable.exe)**:
é só abrir.

O app ainda não tem assinatura digital, então o Windows SmartScreen pode avisar
"O Windows protegeu o computador". Clique em **Mais informações** → **Executar
assim mesmo**. O Windows 10/11 já traz o WebView2 que o app usa; se faltar, o
instalador baixa sozinho.

### Linux

**Ubuntu, Debian, Mint, Pop!_OS** (`.deb`):

```bash
wget https://github.com/Anghievischi/silicio/releases/latest/download/Silicio-amd64.deb
sudo apt install ./Silicio-amd64.deb
```

**Fedora, openSUSE** (`.rpm`):

```bash
sudo dnf install https://github.com/Anghievischi/silicio/releases/latest/download/Silicio-x86_64.rpm
```

**Qualquer distribuição** (AppImage, sem instalar):

```bash
wget https://github.com/Anghievischi/silicio/releases/latest/download/Silicio-x86_64.AppImage
chmod +x Silicio-x86_64.AppImage
./Silicio-x86_64.AppImage
```

**Arch Linux**: use o AppImage acima ou compile a partir do código (seção
[Rodando](#rodando)). O `.deb` também pode ser convertido com o `debtap`.

Depois de instalado, o Silício aparece no menu de aplicativos. Os pacotes `.deb`
e `.rpm` incluem o helper das leituras avançadas e a regra do polkit; o
AppImage não inclui, então soquete, tensão e bus real não aparecem nele.

Para ver os módulos de memória na aba SPD, o driver do SPD precisa estar
carregado. Na maioria das distribuições isso é automático; se a aba ficar
vazia, tente:

```bash
sudo modprobe ee1004      # DDR4
sudo modprobe spd5118     # DDR5
```

## O que já funciona

Aba **Processador**:

- nome, fabricante, codinome, microarquitetura e processo de fabricação;
- família, modelo, stepping e microcódigo;
- clock médio e pico ao vivo, com gráfico dos últimos 60 segundos;
- mapa de núcleos com clock e uso por thread;
- temperatura do pacote;
- caches L1/L2/L3 com associatividade;
- instruções suportadas, agrupadas por categoria;
- multiplicador estimado (bus nominal de 100 MHz);
- botão para copiar o relatório em JSON.

Aba **Placa-mãe**:

- fabricante, produto, família e tipo de chassi;
- modelo e revisão da placa, versão e data do BIOS;
- chipset e southbridge identificados pela base `pci.ids`;
- link PCIe de cada adaptador de vídeo (atual × máximo), com alerta de largura reduzida.

Aba **Memória**:

- total instalado, tipo, canais, velocidade e clock real da DRAM;
- uso de RAM e swap ao vivo;
- cada módulo: slot, canal, fabricante (decodificado pelo código JEDEC), modelo, velocidade, ranks;
- diagnóstico: single channel, módulos abaixo da velocidade nominal (XMP/EXPO desligado) e kits diferentes.

Aba **Módulos (SPD)**, lida direto do EEPROM de cada pente:

- tipo, tamanho, velocidade máxima, largura de banda, tensão, ranks e organização dos chips;
- fabricante do módulo e dos chips, modelo, semana/ano de fabricação, checagem de CRC;
- timings mínimos em ns e tabela de timings nas velocidades JEDEC (CL-tRCD-tRP-tRAS-tRC);
- detecção de XMP. Por enquanto só DDR4; DDR5 é reconhecido mas não decodificado.

Aba **Gráficos**:

- nome, fabricante, integrada ou dedicada, driver, BIOS de vídeo e link PCIe;
- clock do núcleo e da memória, uso, temperatura, consumo, tensão e ventoinha ao vivo;
- VRAM total, compartilhada e em uso.
- AMD e Intel pelo driver do Linux; NVIDIA pela NVML (Linux e Windows).

**Leituras avançadas** (botão "Desbloquear" na aba Processador): soquete, tensão do
núcleo (VID), bus real medido e multiplicador real. Precisam de root, então o app
roda o `silicio-helper` via `pkexec` e o sistema pede a senha uma vez.

Atalhos: **Ctrl+1** a **Ctrl+5** trocam de aba.

A cor de destaque acompanha o fabricante (vermelho para AMD, azul para Intel) e
o tema segue o modo claro/escuro do sistema.

## Rodando

Requisitos: Rust, Node e pnpm. No Linux, também `webkit2gtk-4.1`.

```bash
pnpm install
pnpm tauri dev      # abre o app em modo de desenvolvimento
pnpm tauri build    # gera os pacotes de instalação
```

Relatório em JSON sem abrir a janela:

```bash
cd src-tauri && cargo run -- --json
```

Helper com privilégio, direto no terminal:

```bash
cd src-tauri && cargo build && pkexec "$PWD/target/debug/silicio-helper" | jq
```

Testes do backend:

```bash
cd src-tauri && cargo test
```

## Estrutura

```
src/                         interface (SvelteKit em modo SPA)
  routes/+page.svelte        aba Processador
  routes/placa-mae/          aba Placa-mãe
  routes/memoria/            aba Memória
  routes/spd/                aba Módulos (SPD)
  routes/graficos/           aba Gráficos
  lib/privileged.svelte.ts   estado das leituras com privilégio
  lib/components/            Sidebar, Card, Sparkline
  lib/types.ts               tipos espelhando as structs do Rust
src-tauri/src/
  lib.rs                     comandos Tauri (cpu_*, board_info, memory_*) e modo --json
  hw/cpu.rs                  leitura do processador (CPUID, sysfs, sysinfo)
  hw/codenames.rs            banco próprio de codinomes Intel/AMD
  hw/board.rs                sistema, placa-mãe, BIOS, chipset
  hw/memory.rs               RAM, módulos (SMBIOS via udev), diagnóstico
  hw/pci.rs                  dispositivos PCI, nomes e links PCIe
  hw/gpu.rs                  placas de vídeo (sysfs, NVML, WMI)
  hw/wmi.rs                  consultas WMI no Windows
  hw/spd.rs                  decodificador de SPD DDR4 (JEDEC 21-C)
  hw/privileged.rs           MSR e SMBIOS bruto (só no helper)
  bin/silicio-helper.rs      helper executado como root via pkexec
  tests/fixtures/            SPD real usado nos testes (serial zerado)
packaging/linux/             política do polkit para o helper
docs/analise-cpu-z.md        levantamento de funcionalidades do CPU-Z
```

## Fontes dos dados

| Dado | Linux | Windows |
|------|-------|---------|
| Nome, assinatura, cache, instruções | CPUID (`raw-cpuid`) | CPUID (`raw-cpuid`) |
| Clock e uso por thread | `sysinfo` (cpufreq) | `sysinfo` |
| Faixa de clock, driver, governor | `/sys/devices/system/cpu/cpu0/cpufreq` | pendente |
| Topologia | `/sys/.../topology/core_id` | threads irmãs em sequência |
| Microcódigo | `/proc/cpuinfo` | pendente (registro) |
| Temperatura | hwmon (`k10temp`, `coretemp`) | pendente (PawnIO) |
| Sistema e placa-mãe | `sysinfo` (DMI) | `sysinfo` (SMBIOS) |
| BIOS e chassi | `/sys/class/dmi/id` | WMI (`Win32_BIOS`) |
| Chipset e PCIe | `/sys/bus/pci/devices` + `pci.ids` | pendente |
| Módulos de memória | udev (`/run/udev/data/+dmi:id`), sem root | WMI (`Win32_PhysicalMemory`) |
| Uso de RAM e swap | `sysinfo` | `sysinfo` |
| SPD dos módulos | `ee1004`/`spd5118` no sysfs, sem root | pendente (PawnIO) |
| Tensão, bus, multiplicador | MSR via helper (`pkexec`) | pendente (PawnIO) |
| Soquete, clock externo | SMBIOS tipo 4 via helper | pendente |
| Placa de vídeo | sysfs (`amdgpu`, `i915`, `xe`) + NVML | WMI + NVML |

## Roteiro

1. **Memória avançada**: perfis XMP/EXPO decodificados, SPD DDR5, timings atuais do
   controlador de memória (registradores do UMC na AMD).
2. **Gráficos no Windows**: clocks e uso de AMD e Intel (APIs dos fabricantes).
3. **Windows**: leituras privilegiadas com o driver PawnIO.
4. **Integrações**: card de specs para compartilhar, módulo para Waybar,
   exportação para Prometheus/MQTT, alertas de temperatura.
5. **Benchmark** com ranking online.

## Publicando uma versão

O workflow `.github/workflows/release.yml` gera os instaladores de Windows e
Linux e publica a release quando uma tag é enviada:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

## Aviso legal

O Silício é uma implementação independente. O levantamento em
`docs/analise-cpu-z.md` registra apenas quais informações o CPU-Z exibe; nenhum
código, tabela ou imagem dele foi copiado. CPU-Z é marca da CPUID.

Licença MIT; veja [LICENSE](LICENSE).

