#!/usr/bin/env python3
"""Monta o latest.json do atualizador (ADR-0034) a partir dos arquivos de uma release do GitHub.

Por que existe: cada job de instaladores mesclava o próprio latest.json na release; com os jobs em
paralelo, um sobrescrevia o outro (v0.4.0 saiu só com Linux). Agora os jobs só enviam arquivos e o
job final chama este script, uma vez, com todos os arquivos já na release.

Uso: release_latest_json.py RELEASE.json PASTA_DOS_SIG VERSAO [--out latest.json]
  RELEASE.json   saída de `gh api repos/<dono>/<repo>/releases/<id>` (assets com name e url)
  PASTA_DOS_SIG  pasta com os arquivos .sig baixados da release
Falha (saída 1) se faltar alguma plataforma: a release não é publicada incompleta.
"""
import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

# sufixo do arquivo de instalação → chaves de plataforma que ele atende (mesmas da tauri-action)
TARGETS = [
    ("_x64-setup.exe", ["windows-x86_64", "windows-x86_64-nsis"]),  # NSIS (instala em modo passivo)
    ("_x64_en-US.msi", ["windows-x86_64-msi"]),
    (".AppImage", ["linux-x86_64", "linux-x86_64-appimage"]),
    (".deb", ["linux-x86_64-deb"]),
    (".rpm", ["linux-x86_64-rpm"]),
    (
        "_universal.app.tar.gz",
        ["darwin-aarch64", "darwin-x86_64", "darwin-universal", "darwin-aarch64-app", "darwin-x86_64-app", "darwin-universal-app"],
    ),
]
EXPECTED = sorted(k for _, keys in TARGETS for k in keys)


def build(version: str, notes: str, pub_date: str, assets: list[dict], signatures: dict[str, str]) -> dict:
    """`assets`: [{name, url}]; `signatures`: nome do instalador → conteúdo do .sig."""
    platforms: dict[str, dict] = {}
    for suffix, keys in TARGETS:
        found = [a for a in assets if a["name"].endswith(suffix) and not a["name"].endswith(".sig")]
        if len(found) != 1:
            continue
        sig = signatures.get(found[0]["name"], "").strip()
        if not sig:
            continue
        for k in keys:
            platforms[k] = {"signature": sig, "url": found[0]["url"]}
    return {"version": version, "notes": notes, "pub_date": pub_date, "platforms": platforms}


def missing(doc: dict) -> list[str]:
    return [k for k in EXPECTED if k not in doc["platforms"]]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("release_json")
    ap.add_argument("sig_dir")
    ap.add_argument("version")
    ap.add_argument("--out", default="latest.json")
    a = ap.parse_args()
    rel = json.loads(Path(a.release_json).read_text(encoding="utf-8"))
    assets = [{"name": x["name"], "url": x["url"]} for x in rel.get("assets", [])]
    sigs = {p.name[: -len(".sig")]: p.read_text(encoding="utf-8") for p in Path(a.sig_dir).glob("*.sig")}
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")
    doc = build(a.version.lstrip("v"), rel.get("body") or "", now, assets, sigs)
    gaps = missing(doc)
    if gaps:
        print(f"latest.json incompleto — faltam: {', '.join(gaps)}", file=sys.stderr)
        return 1
    Path(a.out).write_text(json.dumps(doc, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"latest.json {doc['version']} com {len(doc['platforms'])} plataformas")
    return 0


if __name__ == "__main__":
    sys.exit(main())
