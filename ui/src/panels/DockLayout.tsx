import { useCallback, useMemo } from "react";
import { DockviewReact, themeDark, themeLight, type DockviewReadyEvent, type IDockviewPanelProps } from "dockview-react";
import "dockview-react/dist/styles/dockview.css";
import { PANELS } from "./registry";

const LAYOUT_KEY = "appmusica.layout.v1";

function PanelHost(props: IDockviewPanelProps<{ id: string }>) {
  const def = PANELS.find((p) => p.id === props.params.id);
  if (!def) return null;
  const C = def.component;
  return (
    <div className="panel-host">
      <C />
    </div>
  );
}

/** Área de trabalho com painéis encaixáveis; o layout é lembrado por usuário (localStorage). */
export function DockLayout({ theme }: { theme: "dark" | "light" }) {
  const components = useMemo(() => ({ host: PanelHost }), []);
  const onReady = useCallback((e: DockviewReadyEvent) => {
    try {
      const saved = localStorage.getItem(LAYOUT_KEY);
      if (saved) {
        e.api.fromJSON(JSON.parse(saved));
        // painéis novos (adicionados em versões posteriores) entram no grupo inferior
        for (const p of PANELS) if (!e.api.getPanel(p.id)) e.api.addPanel({ id: p.id, component: "host", title: p.title, params: { id: p.id } });
      }
    } catch {
      e.api.clear();
    }
    if (e.api.panels.length === 0) {
      const main = PANELS.filter((p) => p.area === "main");
      const bottom = PANELS.filter((p) => p.area !== "main");
      for (const p of main) e.api.addPanel({ id: p.id, component: "host", title: p.title, params: { id: p.id } });
      bottom.forEach((p, i) => {
        e.api.addPanel({
          id: p.id,
          component: "host",
          title: p.title,
          params: { id: p.id },
          position: i === 0 ? { referencePanel: main[0]!.id, direction: "below" } : { referencePanel: bottom[0]!.id, direction: "within" },
          inactive: i > 0,
        });
      });
      const g = e.api.getPanel(bottom[0]!.id)?.group;
      g?.api.setSize({ height: 330 });
    }
    e.api.onDidLayoutChange(() => {
      try {
        localStorage.setItem(LAYOUT_KEY, JSON.stringify(e.api.toJSON()));
      } catch {
        /* armazenamento indisponível */
      }
    });
  }, []);
  return <DockviewReact className="dock" components={components} onReady={onReady} theme={theme === "dark" ? themeDark : themeLight} />;
}
