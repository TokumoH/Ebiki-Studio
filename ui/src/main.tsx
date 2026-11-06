import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./shared/tokens.css";
import "./shared/ui.css";
import { App } from "./app/App";
import { isTauri, setTransport } from "./ipc/transport";
import { createMockTransport } from "./ipc/mock";
import { withCompositionMock } from "./ipc/mockComposition";
import { hideSplash, splashStatus } from "./app/splash";
import { createMockUpdater, setUpdater } from "./ipc/updater";
import { autoCheckForUpdates } from "./app/updates";

async function boot() {
  splashStatus("Conectando ao motor de áudio…");
  if (isTauri() && !__FORCE_MOCK__) {
    const { tauriTransport } = await import("./ipc/tauri");
    setTransport(tauriTransport);
    const { createTauriUpdater } = await import("./ipc/updaterTauri");
    setUpdater(createTauriUpdater());
  } else {
    setTransport(withCompositionMock(createMockTransport()));
    setUpdater(createMockUpdater());
  }
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
  splashStatus("Pronto");
  // espera o primeiro quadro desenhado pelo React antes de revelar a interface
  requestAnimationFrame(() => requestAnimationFrame(() => void hideSplash()));
  // só se o usuário ligou "Verificar ao abrir" (desligado por padrão: o app segue offline)
  autoCheckForUpdates();
}

void boot();
