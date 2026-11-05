import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./shared/tokens.css";
import "./shared/ui.css";
import { App } from "./app/App";
import { isTauri, setTransport } from "./ipc/transport";
import { createMockTransport } from "./ipc/mock";
import { withCompositionMock } from "./ipc/mockComposition";

async function boot() {
  if (isTauri() && !__FORCE_MOCK__) {
    const { tauriTransport } = await import("./ipc/tauri");
    setTransport(tauriTransport);
  } else {
    setTransport(withCompositionMock(createMockTransport()));
  }
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

void boot();
