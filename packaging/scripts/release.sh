#!/usr/bin/env bash
# Lança uma versão do começo ao fim:
#   1. sobe a versão em package.json, Cargo.toml/Cargo.lock e tauri.conf.json;
#   2. transforma o [Unreleased] do CHANGELOG na versão nova;
#   3. abre o PR "Versão X", espera o CI passar e faz o merge (squash);
#   4. publica a tag vX, que dispara o workflow de release;
#   5. espera a release terminar e atualiza os manifestos (Scoop, winget, AUR) na main.
#
# Uso: packaging/scripts/release.sh 0.5.0
set -euo pipefail

VERSION="${1:?uso: release.sh X.Y.Z}"
VERSION="${VERSION#v}"
TAG="v$VERSION"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "versão inválida: $VERSION" >&2; exit 1; }
[[ -z "$(git status --porcelain)" ]] || { echo "há mudanças não commitadas" >&2; exit 1; }
git checkout -q main
git pull -q --ff-only
git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && { echo "a tag $TAG já existe" >&2; exit 1; }
grep -q '^## \[Unreleased\]' CHANGELOG.md || { echo "CHANGELOG sem [Unreleased]" >&2; exit 1; }

PREVIOUS="$(git describe --tags --abbrev=0 --match 'v*')"
echo "==> $PREVIOUS -> $TAG"

git checkout -q -b "release/$VERSION"
python3 - "$VERSION" "${PREVIOUS#v}" <<'PY'
import json, re, sys
from pathlib import Path

version, previous = sys.argv[1], sys.argv[2]

for path in ("package.json", "src-tauri/tauri.conf.json"):
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    p.write_text(re.sub(r'("version":\s*")[^"]+(")', rf"\g<1>{version}\g<2>", text, count=1), encoding="utf-8")

cargo = Path("src-tauri/Cargo.toml")
cargo.write_text(re.sub(r'(?m)^version = "[^"]+"', f'version = "{version}"', cargo.read_text(encoding="utf-8"), count=1), encoding="utf-8")

changelog = Path("CHANGELOG.md")
text = changelog.read_text(encoding="utf-8")
text = text.replace("## [Unreleased]\n", f"## [Unreleased]\n\n## [{version}]\n", 1)
repo = "https://github.com/Anghievischi/silicio"
text = re.sub(
    r"(?m)^\[Unreleased\]: .*$",
    f"[Unreleased]: {repo}/compare/v{version}...HEAD\n[{version}]: {repo}/compare/v{previous}...v{version}",
    text,
    count=1,
)
changelog.write_text(text, encoding="utf-8")
PY
(cd src-tauri && cargo update -q -p silicio --offline 2>/dev/null || cargo check -q)

git add -A
git commit -q -m "Versão $VERSION"
git push -q -u origin "release/$VERSION"
gh pr create --base main --title "Versão $VERSION" \
  --body "Sobe a versão para $VERSION e fecha o CHANGELOG. Depois do merge, a tag $TAG dispara a release."

echo "==> esperando o CI do PR"
sleep 20
gh pr checks "release/$VERSION" --watch --interval 30 --fail-fast
gh pr merge "release/$VERSION" --squash --delete-branch

git checkout -q main
git pull -q --ff-only
git tag "$TAG"
git push -q origin "$TAG"

echo "==> esperando o workflow de release"
sleep 20
RUN="$(gh run list --workflow release.yml --branch "$TAG" --limit 1 --json databaseId -q '.[0].databaseId')"
gh run watch "$RUN" --interval 60 --exit-status >/dev/null

echo "==> atualizando os manifestos"
python3 packaging/scripts/update-manifests.py "$VERSION"
git add -A bucket packaging
git commit -q -m "Manifestos da $VERSION"
git push -q origin main

cat <<MSG

Versão $VERSION publicada: https://github.com/Anghievischi/silicio/releases/tag/$TAG
Falta enviar fora do GitHub:
  - winget:  wingetcreate submit packaging/winget/$VERSION
  - AUR:     copiar packaging/aur/PKGBUILD e .SRCINFO para o repositório silicio-bin e dar push
MSG
