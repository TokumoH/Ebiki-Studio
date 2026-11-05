import { useEffect } from "react";
import "./app.css";
import { useSession } from "../core/session";
import { startTelemetry } from "../core/live";
import { applyTheme, useTheme } from "../core/theme";
import { StartScreen } from "./StartScreen";
import { Workspace } from "./Workspace";
import { Toasts } from "./Toasts";

export function App() {
  const hasProject = useSession((s) => s.session !== null);
  const theme = useTheme((s) => s.theme);
  useEffect(() => applyTheme(theme), [theme]);
  useEffect(() => {
    let off: (() => void) | undefined;
    void startTelemetry().then((f) => (off = f));
    return () => off?.();
  }, []);
  return (
    <>
      {hasProject ? <Workspace /> : <StartScreen />}
      <Toasts />
    </>
  );
}
