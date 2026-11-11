#!/usr/bin/env bash
# Executa localmente as mesmas verificações da CI (.github/workflows/ci.yml).
# Uso:  scripts/ci-local.sh          (tudo)
#       scripts/ci-local.sh rust     (só Rust)
#       scripts/ci-local.sh ui       (só interface)
#       scripts/ci-local.sh docs     (só documentação: regra de READMEs)
# Saída 0 = tudo verde. Útil quando os minutos do GitHub Actions acabam.
set -euo pipefail
cd "$(dirname "$0")/.."

what="${1:-all}"
step() { printf '\n\033[1;35m▶ %s\033[0m\n' "$*"; }

if [[ "$what" == all || "$what" == rust ]]; then
  # o app Tauri exige ui/dist para compilar; cria um placeholder se ainda não houve build
  [[ -f ui/dist/index.html ]] || { mkdir -p ui/dist && echo '<!doctype html>' > ui/dist/index.html; }
  step "Formatação";            cargo fmt --all --check
  step "Clippy";                cargo clippy --workspace --all-targets -- -D warnings
  step "Testes Rust";           cargo test --workspace
  step "Desempenho DSP (release)"; cargo test --release -p dsp --test performance
  step "Simulação de sessão (release)"; cargo test --release -p infra-adapters --test simulation -- --nocapture
  step "Contrato IPC";          cargo test -p app-core --test contract_fixtures
  git diff --exit-code -- ui/src/ipc/__fixtures__ \
    || { echo "Fixtures do contrato mudaram: revise e faça commit."; exit 1; }
fi

if [[ "$what" == all || "$what" == docs ]]; then
  step "READMEs (CLAUDE.md §6)"; python3 scripts/check_readmes.py
fi

if [[ "$what" == all || "$what" == ui ]]; then
  [[ -d ui/node_modules ]] || { step "npm ci"; npm --prefix ui ci; }
  step "Tipos (tsc)";           (cd ui && npx tsc -b --noEmit)
  step "Testes unitários";      (cd ui && npx vitest run)
  step "Build da interface";    (cd ui && npx vite build)
  step "E2E (Playwright)";      (cd ui && npx playwright install chromium >/dev/null && npx playwright test --trace off)
fi

printf '\n\033[1;32m✔ Verificação local concluída sem erros.\033[0m\n'
