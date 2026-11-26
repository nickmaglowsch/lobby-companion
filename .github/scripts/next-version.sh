#!/usr/bin/env bash
# Próxima versão para o auto-release.yml (roda no ubuntu do Actions, com gh, jq, node e perl).
# Saídas em $GITHUB_OUTPUT: tag (vazia = nada a lançar), changes (lista de novidades), dry.
# Com DRY_RUN=true só mostra o que faria: nada de commit, tag ou push.
set -euo pipefail

out="${GITHUB_OUTPUT:-/dev/stdout}"
dry="${DRY_RUN:-false}"
echo "dry=$dry" >> "$out"

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
# Sempre a main mais nova: se outra mesclagem chegou enquanto esperava na fila, entra nesta versão.
git fetch --tags --force origin main
git checkout -q -B main origin/main

if git tag --points-at HEAD | grep -q '^v[0-9]'; then
  echo "a main já tem tag ($(git tag --points-at HEAD | tr '\n' ' ')): nada a lançar"
  echo "tag=" >> "$out"
  exit 0
fi

last=$(git tag -l 'v[0-9]*' --sort=-v:refname | head -1)
lastv=${last#v}
lastv=${lastv:-0.0.0}
cur=$(node -p "require('./src-tauri/tauri.conf.json').version")
range="${last:+$last..}HEAD"

# Novidades: título dos PRs mesclados desde a última tag; o nível sobe com label minor/major.
level=patch
changes=""
for n in $(git log --format=%s "$range" | sed -nE 's/^Merge pull request #([0-9]+).*/\1/p'); do
  title=$(gh pr view "$n" --json title --jq .title)
  labels=$(gh pr view "$n" --json labels --jq '[.labels[].name] | join(",")')
  case ",$labels," in
    *,major,*) level=major ;;
    *,minor,*) [ "$level" = patch ] && level=minor ;;
  esac
  case "$title" in "Versão "*) continue ;; esac
  changes+="- $title (#$n)"$'\n'
done
if [ -z "$changes" ]; then
  changes=$(git log --no-merges --format='- %s' "$range" | grep -vE '^- Versão [0-9]' | head -20 || true)
fi
[ -n "$changes" ] || changes="- Correções e melhorias internas."

newer() { [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)" = "$1" ]; }
bump=true
if newer "$cur" "$lastv"; then
  version=$cur
  bump=false
else
  IFS=. read -r ma mi pa <<<"$lastv"
  case $level in
    major) version="$((ma + 1)).0.0" ;;
    minor) version="$ma.$((mi + 1)).0" ;;
    *) version="$ma.$mi.$((pa + 1))" ;;
  esac
fi

echo "última tag: ${last:-nenhuma} · versão nos arquivos: $cur · nível: $level · nova: $version (trocar arquivos: $bump)"
echo "novidades:"
printf '%s\n' "$changes"

if [ "$dry" = "true" ]; then
  echo "tag=" >> "$out"
else
  if [ "$bump" = true ]; then
    npm version "$version" --no-git-tag-version --allow-same-version > /dev/null
    sed -i -E "0,/\"version\": \"[^\"]+\"/s//\"version\": \"$version\"/" src-tauri/tauri.conf.json
    sed -i -E "0,/^version = \"[^\"]+\"/s//version = \"$version\"/" src-tauri/Cargo.toml
    perl -0pi -e "s/(name = \"lobby-companion\"\nversion = )\"[^\"]+\"/\${1}\"$version\"/" src-tauri/Cargo.lock
    git diff --stat
    git commit -qam "Versão $version [skip ci]"
    git push origin HEAD:main
  fi
  git tag -a "v$version" -m "v$version"
  git push origin "v$version"
  echo "tag=v$version" >> "$out"
fi
{
  echo "changes<<EOF_CHANGES"
  printf '%s\n' "$changes"
  echo "EOF_CHANGES"
} >> "$out"
