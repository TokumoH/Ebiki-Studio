import { useCallback, useEffect, useMemo, useRef } from "react";
import { DockviewReact, themeDark, themeLight, type DockviewApi, type DockviewReadyEvent, type IDockviewPanelProps } from "dockview-react";
import "dockview-react/dist/styles/dockview.css";
import { PANELS } from "./registry";
import { attachDock, panelOptions } from "./dockControl";

const LAYOUT_KEY = "ebiki.layout.v1";

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
  const dockApi = useRef<DockviewApi | null>(null);
  // simétrico (StrictMode monta/desmonta/remonta): conecta ao montar, desconecta ao desmontar
  useEffect(() => {
    attachDock(dockApi.current);
    return () => attachDock(null);
  }, []);
  const onReady = useCallback((e: DockviewReadyEvent) => {
    try {
      const saved = localStorage.getItem(LAYOUT_KEY);
      if (saved) {
        e.api.fromJSON(JSON.parse(saved));
        // painéis novos (adicionados em versões posteriores) entram junto aos da mesma área;
        // os que o usuário fechou continuam fechados (reabertos pelo menu "Painéis")
        const known = new Set<string>(JSON.parse(localStorage.getItem(`${LAYOUT_KEY}.known`) ?? "[]"));
        for (const p of PANELS) {
          if (e.api.getPanel(p.id) || known.has(p.id)) continue;
          const opts = panelOptions(e.api, p.id);
          if (opts) e.api.addPanel({ ...opts, inactive: true });
        }
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
    const persist = () => {
      try {
        localStorage.setItem(LAYOUT_KEY, JSON.stringify(e.api.toJSON()));
        localStorage.setItem(`${LAYOUT_KEY}.known`, JSON.stringify(PANELS.map((p) => p.id)));
      } catch {
        /* armazenamento indisponível */
      }
    };
    persist();
    e.api.onDidLayoutChange(persist);
    dockApi.current = e.api;
    attachDock(e.api);
  }, []);
  return <DockviewReact className="dock" components={components} onReady={onReady} theme={theme === "dark" ? themeDark : themeLight} />;
}
