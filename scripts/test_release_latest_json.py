#!/usr/bin/env python3
"""Testes do release_latest_json.py: reproduz o latest.json da v0.3.0 (publicado e conferido) a partir
da lista de arquivos daquela release, e recusa release incompleta (o caso da v0.4.0)."""
import importlib.util
import sys
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("rlj", Path(__file__).with_name("release_latest_json.py"))
rlj = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rlj)

API = "https://api.github.com/repos/TokumoH/Ebiki-Studio/releases/assets/"
# arquivos da release v0.3.0 (nome → id do asset)
ASSETS_030 = {
    "Ebiki.Studio-0.3.0-1.x86_64.rpm": 733540890,
    "Ebiki.Studio_0.3.0_amd64.AppImage": 733540976,
    "Ebiki.Studio_0.3.0_amd64.deb": 733540797,
    "Ebiki.Studio_0.3.0_universal.app.tar.gz": 733552210,
    "Ebiki.Studio_0.3.0_universal.dmg": 733552085,
    "Ebiki.Studio_0.3.0_x64-setup.exe": 733538112,
    "Ebiki.Studio_0.3.0_x64_en-US.msi": 733538039,
    "Ebiki.Studio_0.3.0_x64_pt-BR.msi": 733537955,
}
# latest.json da v0.3.0: plataforma → asset (conferido na publicação, VER-2026-11-11-03)
EXPECTED_030 = {
    "windows-x86_64": 733538112, "windows-x86_64-nsis": 733538112, "windows-x86_64-msi": 733538039,
    "linux-x86_64": 733540976, "linux-x86_64-appimage": 733540976, "linux-x86_64-deb": 733540797,
    "linux-x86_64-rpm": 733540890, "darwin-aarch64": 733552210, "darwin-x86_64": 733552210,
    "darwin-universal": 733552210, "darwin-aarch64-app": 733552210, "darwin-x86_64-app": 733552210,
    "darwin-universal-app": 733552210,
}


def assets(names):
    out = [{"name": n, "url": API + str(ASSETS_030[n])} for n in names]
    out += [{"name": n + ".sig", "url": API + "0"} for n in names if not n.endswith(".dmg")]
    return out


class Test(unittest.TestCase):
    def test_reproduz_o_latest_json_da_030(self):
        sigs = {n: f"assinatura-de-{n}" for n in ASSETS_030}
        doc = rlj.build("0.3.0", "notas", "2026-11-11T00:45:37.000Z", assets(ASSETS_030), sigs)
        self.assertEqual(rlj.missing(doc), [])
        self.assertEqual({k: v["url"] for k, v in doc["platforms"].items()}, {k: API + str(v) for k, v in EXPECTED_030.items()})
        self.assertEqual(doc["platforms"]["windows-x86_64-msi"]["signature"], "assinatura-de-Ebiki.Studio_0.3.0_x64_en-US.msi")
        self.assertEqual(doc["version"], "0.3.0")

    def test_release_so_com_linux_e_recusada(self):
        linux = [n for n in ASSETS_030 if n.endswith((".rpm", ".deb", ".AppImage"))]
        doc = rlj.build("0.4.0", "", "x", assets(linux), {n: "s" for n in linux})
        self.assertIn("windows-x86_64", rlj.missing(doc))
        self.assertIn("darwin-universal", rlj.missing(doc))

    def test_instalador_sem_assinatura_nao_entra(self):
        sigs = {n: "s" for n in ASSETS_030 if not n.endswith(".exe")}
        doc = rlj.build("0.3.0", "", "x", assets(ASSETS_030), sigs)
        self.assertIn("windows-x86_64-nsis", rlj.missing(doc))


if __name__ == "__main__":
    sys.exit(unittest.main())
