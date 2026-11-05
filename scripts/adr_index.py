#!/usr/bin/env python3
"""Regenera docs/adr/README.md a partir dos arquivos ADR (título e status)."""
import os
import re

D = os.path.join(os.path.dirname(__file__), "..", "docs", "adr")
rows = []
for f in sorted(os.listdir(D)):
    m = re.match(r"(\d{4})-.*\.md$", f)
    if not m:
        continue
    text = open(os.path.join(D, f), encoding="utf-8").read()
    title = re.sub(r"^#\s*ADR-\d+:\s*", "", text.splitlines()[0].strip())
    st = re.search(r"\*\*Status:\*\*\s*(.+)", text)
    rows.append(f"| [{m.group(1)}]({f}) | {title} | {st.group(1).strip() if st else '—'} |")
with open(os.path.join(D, "README.md"), "w", encoding="utf-8") as out:
    out.write("# Architecture Decision Records\n\nGerado por `scripts/adr_index.py`.\n\n| ADR | Decisão | Status |\n|---|---|---|\n")
    out.write("\n".join(rows) + "\n")
