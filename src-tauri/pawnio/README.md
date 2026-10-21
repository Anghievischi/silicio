# Módulos PawnIO

Módulos assinados do [PawnIO.Modules](https://github.com/namazso/PawnIO.Modules),
release 0.2.11, sem modificação. Licença LGPL-2.1 (ver `COPYING`); o código-fonte
está no repositório deles (`AMDFamily17.p`, `IntelMSR.p`).

O Silício só os carrega no driver [PawnIO](https://pawnio.eu) pela interface de
IOCTL, no Windows, para ler MSRs (Intel e AMD) e o sensor de temperatura das CPUs
AMD Zen (SMN).

Para atualizar, baixe o `release_X.zip` da página de releases e substitua os
arquivos `.bin`.
