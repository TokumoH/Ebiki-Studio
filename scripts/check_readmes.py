#!/usr/bin/env python3
"""Confere a regra de READMEs do CLAUDE.md (§6): todo diretório tem README ou é coberto pelo README do módulo.

- Cada diretório versionado (fora de ocultos, build e dependências) tem README.md próprio OU aparece
  como `subpasta/` na tabela "Onde" do README do ancestral mais próximo.
- Todo arquivo de um diretório coberto aparece em "Onde" (caminho relativo ao README).
- Links relativos dos READMEs abrem.
Saída 0 = ok; 1 = lista os problemas.
"""
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SKIP = {"target", "node_modules", "dist", "gen", "test-results", "playwright-report", "__pycache__"}


def onde(readme: Path) -> set[str]:
    """Itens listados: links da seção "Onde"; READMEs mais antigos (índice de ADRs, marca, manual)
    não têm a seção, e aí vale qualquer link relativo do arquivo."""
    text = readme.read_text(encoding="utf-8")
    section = text.split("## Onde", 1)[1].split("\n## ", 1)[0] if "## Onde" in text else text
    links = re.findall(r"\]\(([^)\s#]+)", section)
    return {l.removeprefix("./").rstrip("/") for l in links if not l.startswith(("http", "mailto:"))}


def tracked() -> tuple[set[Path], set[Path]]:
    """Arquivos e diretórios versionados (artefatos ignorados pelo git não contam)."""
    out = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split("\n")
    files = {ROOT / f for f in out if f}
    dirs = {p for f in files for p in f.parents if ROOT in p.parents or p == ROOT}
    return files, dirs


def main() -> int:
    problems: list[str] = []
    tracked_files, tracked_dirs = tracked()
    readmes: dict[Path, set[str]] = {}
    for dirpath, dirnames, _ in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP and not d.startswith(".")]
        r = Path(dirpath) / "README.md"
        if r.exists():
            readmes[Path(dirpath)] = onde(r)

    def owner(d: Path) -> Path | None:
        p = d
        while True:
            if p in readmes:
                return p
            if p == ROOT:
                return None
            p = p.parent

    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP and not d.startswith(".")]
        d = Path(dirpath)
        if d not in tracked_dirs:
            continue
        filenames = [f for f in filenames if d / f in tracked_files]
        own = owner(d)
        if own is None:
            problems.append(f"{d.relative_to(ROOT)}: sem README e sem README de módulo acima")
            continue
        listed = readmes[own]
        if own != d and str(d.relative_to(own)) not in listed:
            problems.append(f"{d.relative_to(ROOT)}: não tem README e não aparece em {own.relative_to(ROOT) / 'README.md'}")
        # o README da raiz do repositório é a vitrine do projeto (não lista arquivos)
        if own == ROOT:
            continue
        for f in filenames:
            if f == "README.md" or f.startswith("."):
                continue
            rel = str((d / f).relative_to(own))
            if rel not in listed:
                problems.append(f"{(d / f).relative_to(ROOT)}: não listado em {own.relative_to(ROOT) / 'README.md'}")

    for d in readmes:
        text = (d / "README.md").read_text(encoding="utf-8")
        for link in re.findall(r"\]\(([^)\s#]+)", text):
            if link.startswith(("http", "mailto:")):
                continue
            target = (d / link) if not link.startswith("/") else ROOT / link.lstrip("/")
            if not target.exists():
                problems.append(f"{(d / 'README.md').relative_to(ROOT)}: link quebrado {link}")

    if problems:
        print(f"{len(problems)} problema(s) de README:")
        print("\n".join(sorted(problems)))
        return 1
    print(f"READMEs ok ({len(readmes)} arquivos).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
