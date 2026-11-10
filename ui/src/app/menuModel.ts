// Modelo do menu principal: função pura do estado da sessão + ações, para a MenuBar desenhar.
// Os atalhos exibidos vêm dos atalhos configurados (o mesmo registro que o teclado usa).
import type { MenuDef, MenuEntry } from "../shared/MenuBar";
import type { Shortcut } from "./shortcuts";
import { PANEL_CATALOG } from "../panels/catalog";

export interface MenuContext {
  shortcuts: Shortcut[];
  /** Executa a ação de mesmo id do registro de comandos do Workspace. */
  run: (id: string) => void;
  canUndo: boolean;
  canRedo: boolean;
  undoLabel?: string | null;
  redoLabel?: string | null;
  theme: "dark" | "light";
  openPanels: string[];
  statusBar: boolean;
  /** Legendas ao passar o mouse ligadas. */
  tooltips: boolean;
  metronome: boolean;
  loop: boolean;
  hasSelectedTrack: boolean;
  selectedTrackArmed: boolean;
  selectedTrackSpectrum: boolean;
  /** Clipe selecionado (ou null) e seus estados de edição. */
  clip: { polarityInverted: boolean; reversed: boolean } | null;
}

export function buildMenus(c: MenuContext): MenuDef[] {
  const keys = (id: string) => c.shortcuts.find((s) => s.id === id)?.keys;
  const item = (id: string, label: string, extra: Partial<Extract<MenuEntry, { kind: "item" }>> = {}): MenuEntry => ({
    kind: "item",
    id,
    label,
    keys: keys(id),
    run: () => c.run(id),
    ...extra,
  });
  const sep: MenuEntry = { kind: "separator" };
  const panelItems = (prefix: string): MenuEntry[] =>
    PANEL_CATALOG.map((p) => ({
      kind: "item" as const,
      id: `${prefix}:${p.id}`,
      label: p.title,
      checked: c.openPanels.includes(p.id),
      run: () => c.run(`togglePanel:${p.id}`),
    }));
  const noClip = c.clip === null;

  return [
    {
      id: "arquivo",
      label: "Arquivo",
      items: [
        item("newProject", "Novo projeto…"),
        item("openProject", "Abrir projeto…"),
        item("save", "Salvar"),
        sep,
        item("import", "Importar áudio…"),
        item("export", "Exportar…"),
        sep,
        item("audioSettings", "Configurações de áudio…"),
        sep,
        item("closeProject", "Fechar projeto"),
      ],
    },
    {
      id: "editar",
      label: "Editar",
      items: [
        item("undo", c.undoLabel ? `Desfazer: ${c.undoLabel}` : "Desfazer", { disabled: !c.canUndo }),
        item("redo", c.redoLabel ? `Refazer: ${c.redoLabel}` : "Refazer", { disabled: !c.canRedo }),
        sep,
        item("split", "Dividir clipe no cursor"),
        item("deleteClip", "Excluir clipe selecionado", { keys: "Delete", disabled: noClip }),
        sep,
        item("shortcutsDialog", "Atalhos de teclado…"),
      ],
    },
    {
      id: "exibir",
      label: "Exibir",
      items: [
        item("themeDark", "Tema escuro (Odoriko)", { checked: c.theme === "dark", radio: true }),
        item("themeLight", "Tema claro (Agridoce)", { checked: c.theme === "light", radio: true }),
        sep,
        item("zoomIn", "Aproximar"),
        item("zoomOut", "Afastar"),
        item("zoomReset", "Zoom padrão"),
        sep,
        { kind: "submenu", id: "exibir-paineis", label: "Painéis", items: panelItems("view") },
        item("toggleSpectrum", "Espectrograma na faixa selecionada", { checked: c.selectedTrackSpectrum, disabled: !c.hasSelectedTrack }),
        item("toggleStatusBar", "Barra de status", { checked: c.statusBar }),
        item("toggleTooltips", "Legendas ao passar o mouse", { checked: c.tooltips }),
        sep,
        item("fullscreen", "Tela cheia"),
        item("resetLayout", "Restaurar layout padrão"),
      ],
    },
    {
      id: "transporte",
      label: "Transporte",
      items: [
        item("playPause", "Tocar / pausar"),
        item("stopReturn", "Parar e voltar ao início"),
        item("record", "Gravar"),
        item("home", "Voltar ao início"),
        sep,
        item("metronome", "Metrônomo", { checked: c.metronome }),
        item("loop", "Loop de 4 compassos", { checked: c.loop }),
      ],
    },
    {
      id: "faixa",
      label: "Faixa",
      items: [
        item("newTrack", "Nova faixa"),
        item("armTrack", "Armar faixa selecionada", { checked: c.selectedTrackArmed, disabled: !c.hasSelectedTrack }),
        item("deleteTrack", "Excluir faixa selecionada", { disabled: !c.hasSelectedTrack }),
      ],
    },
    {
      id: "clipe",
      label: "Clipe",
      items: [
        item("split", "Dividir no cursor"),
        item("normalizeClip", "Normalizar (−1 dBFS)", { disabled: noClip }),
        item("polarityClip", "Inverter fase", { checked: c.clip?.polarityInverted ?? false, disabled: noClip }),
        item("reverseClip", "Reverter", { checked: c.clip?.reversed ?? false, disabled: noClip }),
        sep,
        item("deleteClip", "Excluir", { keys: "Delete", disabled: noClip }),
      ],
    },
    {
      id: "ferramentas",
      label: "Ferramentas",
      items: [
        item("panel:advisor", "Análise da mixagem", { keys: undefined }),
        item("panel:tuning", "Afinador e correção de afinação", { keys: undefined }),
        item("panel:harmony", "Harmonia e progressões", { keys: undefined }),
        item("panel:bases", "Biblioteca de bases", { keys: undefined }),
        item("panel:score", "Partitura e tablatura", { keys: undefined }),
        item("panel:lyrics", "Letra sincronizada", { keys: undefined }),
        item("panel:tempo", "Mapa de tempo", { keys: undefined }),
      ],
    },
    {
      id: "janela",
      label: "Janela",
      items: [...panelItems("window"), sep, item("resetLayout", "Restaurar layout padrão")],
    },
    {
      id: "ajuda",
      label: "Ajuda",
      items: [
        item("help", "Central de ajuda"),
        item("tutorial", "Tutorial de início"),
        item("shortcutsDialog", "Atalhos de teclado…"),
        sep,
        item("checkUpdates", "Verificar atualizações…"),
        item("about", "Sobre o Ebiki Studio"),
      ],
    },
  ];
}
