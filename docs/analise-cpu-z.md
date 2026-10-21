# Análise do CPU-Z 3.01

Estudo feito com o REA (análise estática, sem executar o programa) para levantar
**o que** o CPU-Z mostra. O Silício é escrito do zero: nenhum código, tabela ou
imagem do CPU-Z é copiado.

- Arquivo: `cpuz_x64.exe` (pacote `cpu-z_3.01-en.zip`, agosto de 2026)
- SHA-256: `a01369043c9e199747d62b950fb6db3f793ea74feb9c667ae291988405905fbe`
- Formato: PE32+ x86-64, interface em MFC (Win32)

## Recursos embutidos

| Tipo | Qtd. | O que é |
|------|------|---------|
| RT_DIALOG | 16 | Uma caixa de diálogo por aba e por janela auxiliar |
| RT_BITMAP | 257 | Logotipos de fabricantes, processadores e chipsets |
| RT_RCDATA | 3 | Três executáveis PE (`MZ`) embutidos: os drivers de kernel usados para ler MSR, SMBus e PCI |
| RT_STRING | 13 | Mensagens padrão do MFC |

Os três drivers embutidos confirmam o que se esperava: o acesso de baixo nível
no Windows depende de um driver próprio. No Silício isso vai usar o **PawnIO**
(o WinRing0 é marcado como vulnerável pelo Windows Defender).

## Campos por aba

**CPU**: Name, Code Name, Package, Technology, Core Voltage, Specification,
Family, Model, Stepping, Ext. Family, Ext. Model, Revision, Instructions,
Clocks (Core Speed, Multiplier, Bus Speed, Rated FSB), Cache (L1 Data, L1 Inst.,
Level 2, Level 3), Selection (processador), Cores, Threads.

**Mainboard**: Manufacturer, Model, Bus Specs., Chipset, Southbridge, LPCIO,
BIOS (Brand, Version, Date), CPU Microcode, Graphic Interface (Bus, Transfer
Rate, Max. Supported, Side Band Addressing).

**Memory**: Type, Size, Channel #, DC Mode, Uncore Frequency, DRAM Frequency,
FSB:DRAM, CAS# Latency (CL), tRCD, tRP, tRAS, tRC, Command Rate (CR),
DRAM Idle Timer, Total CAS# (tRDRAM), Row To Column (tRCD).

**SPD**: Memory Slot Selection, Module Size, Max Bandwidth, Module Manuf.,
DRAM Manuf., Part Number, Serial Number, Week/Year, Buffered/Registered,
Correction, SPD Ext., Timings Table (Frequency, CL, tRCD, tRP, tRAS, tRC,
Command Rate, Voltage) e Profile #1–#4 (XMP/EXPO).

**Graphics**: Display Device Selection, Perf Level, Name, Code Name, Board
Manuf., Revision, Technology, TDP, Cores, ROPs/TMUs, Clocks (GFX Core,
Shader/SoC, Memory), Memory (Size, Type, Vendor, Bus Width).

**Bench**: CPU Single Thread, CPU Multi Thread, Multi Thread Ratio, referência
para comparação, Stress CPU, Submit and Compare.

**About / Tools**: Save Report (.TXT), Validation, Clocks, Timers, Driver Update,
Windows Version, DirectX.

## Janelas auxiliares

- **Validation V3**: envio online da configuração, com modo privado e modo XOC
  (overclock extremo).
- **Update**: checagem de nova versão ao iniciar.
- **Clocks**: janela com o clock de cada núcleo.
- **Timers**: quatro cronômetros (útil para testes de estabilidade).
- **Errors**: lista de erros com opção de salvar.

## O que o Silício já cobre (aba CPU)

| CPU-Z | Silício |
|-------|---------|
| Name, Code Name, Technology | Sim (banco de codinomes próprio) |
| Family, Model, Stepping, Ext. | Sim |
| Microcode | Sim (Linux) |
| Instructions | Sim, agrupadas por categoria |
| Cache L1/L2/L3 | Sim, com associatividade e linha |
| Cores / Threads | Sim, com mapa de topologia |
| Core Speed | Sim, por thread, ao vivo, com histórico |
| Temperatura | **Extra** (o CPU-Z não mostra) |
| Uso por thread | **Extra** |
| Exportar JSON | **Extra** (o CPU-Z só gera .TXT) |
| Multiplier, Bus Speed | Sim: bus medido (TSC ÷ multiplicador base via MSR); estimado sem privilégio |
| Package, Core Voltage | Sim, via helper com privilégio (SMBIOS tipo 4 e MSR) |

## O que o Silício já cobre (abas Mainboard e Memory)

| CPU-Z | Silício |
|-------|---------|
| Manufacturer, Model, Rev. | Sim |
| BIOS Brand, Version, Date | Sim (Linux) |
| Chipset, Southbridge | Sim, pelo `pci.ids` |
| LPCIO | Pendente: exige acesso às portas de I/O |
| Graphic Interface (Bus, Transfer Rate, Max. Supported) | Sim, por adaptador |
| Memory Type, Size, Channel # | Sim |
| DRAM Frequency | Sim (derivada da velocidade SMBIOS) |
| Timings atuais do controlador (CL, tRCD…) | Pendente: exige registradores do controlador |
| SPD: Module Manuf., DRAM Manuf., Part Number, Size, Week/Year | Sim (DDR4) |
| SPD: Max Bandwidth, Ranks, Voltage, Timings Table | Sim (DDR4), conferido com `decode-dimms` |
| SPD: XMP/EXPO Profiles | Sim: XMP 2.0 (DDR4), XMP 3.0 e EXPO (DDR5), com tensão e timings |
| SPD: DDR5 | Sim (Linux, driver `spd5118`) |
| Uso de RAM ao vivo | **Extra** |
| Diagnóstico (single channel, XMP desligado) | **Extra** |
