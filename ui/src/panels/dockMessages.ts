// Textos do dockview em PT-BR (UX-16): botão fechar aba e anúncios para leitor de tela.
import type { DockviewMessages } from "dockview-core";

const onde = (position: string, target: string) =>
  position === "center" ? `junto de ${target}` : `à ${{ left: "esquerda", right: "direita", top: "cima", bottom: "baixo" }[position] ?? position} de ${target}`;

export const DOCK_MESSAGES: Partial<DockviewMessages> = {
  panelOpened: (title) => `${title} aberto`,
  panelClosed: (title) => `${title} fechado`,
  groupMaximized: (title) => `${title} maximizado`,
  groupRestored: (title) => `${title} restaurado`,
  groupFloated: (title) => `${title} solto`,
  groupDocked: (title) => `${title} encaixado`,
  groupPoppedOut: (title) => `${title} aberto em outra janela`,
  closeTab: (title) => `Fechar ${title}`,
  closeTabPlain: () => "Fechar",
  movePickTarget: (source, target, current, total) => `Movendo ${source}. Destino ${target}, ${current} de ${total}. Enter escolhe onde, Esc cancela.`,
  movePickEdge: (position, target) => `${onde(position, target)}. Setas mudam, Enter confirma, Esc volta.`,
  moveCommitted: (source, target, position) => `${source} ${onde(position, target)}.`,
  moveCancelled: () => "Movimento cancelado.",
  moveNotAllowed: () => "Esse movimento não é permitido.",
  moveFloated: (source) => `${source} solto.`,
};
