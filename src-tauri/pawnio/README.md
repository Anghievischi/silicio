# Módulos PawnIO

Módulos assinados do [PawnIO.Modules](https://github.com/namazso/PawnIO.Modules),
release 0.2.11, sem modificação. Licença LGPL-2.1 (ver `COPYING`); o código-fonte
está no repositório deles (`AMDFamily17.p`, `IntelMSR.p`).

O Silício só os carrega no driver [PawnIO](https://pawnio.eu) pela interface de
IOCTL, no Windows, para ler MSRs (Intel e AMD) e o sensor de temperatura das CPUs
AMD Zen (SMN).

Para atualizar, baixe o `release_X.zip` da página de releases e substitua os
arquivos `.bin`.

## Instalador do driver

`packaging/windows/PawnIO_setup.exe` é o instalador oficial do
[PawnIO.Setup](https://github.com/namazso/PawnIO.Setup/releases) 2.2.0, sem
modificação (o próprio instalador diz que pode ser redistribuído assim).
SHA-256 `1f519a22e47187f70a1379a48ca604981c4fcf694f4e65b734aaa74a9fba3032`.
O instalador NSIS do Silício o oferece no final (`installer-hooks.nsh`) e o
roda com `-install -silent`.
