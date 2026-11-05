import { useState } from "react";
import { Button, Dialog } from "../shared/components";
import { eventKeys, loadShortcuts, saveShortcut, type Shortcut } from "./shortcuts";

export function ShortcutsDialog({ open, onClose, onChanged }: { open: boolean; onClose: () => void; onChanged: (s: Shortcut[]) => void }) {
  const [list, setList] = useState(loadShortcuts);
  const [capturing, setCapturing] = useState<string | null>(null);
  return (
    <Dialog title="Atalhos de teclado" open={open} onClose={onClose} footer={<Button onClick={onClose}>Fechar</Button>}>
      <table style={{ width: "100%", borderCollapse: "collapse" }}>
        <tbody>
          {list.map((s) => (
            <tr key={s.id}>
              <td style={{ padding: 4 }}>{s.label}</td>
              <td style={{ padding: 4, textAlign: "right" }}>
                <Button
                  size="sm"
                  aria-label={`Alterar atalho: ${s.label}`}
                  onKeyDown={(e) => {
                    if (capturing !== s.id) return;
                    e.preventDefault();
                    e.stopPropagation();
                    const keys = eventKeys(e.nativeEvent);
                    if (["Ctrl", "Shift", "Alt", "Ctrl+Shift"].includes(keys)) return;
                    saveShortcut(s.id, keys);
                    const next = loadShortcuts();
                    setList(next);
                    onChanged(next);
                    setCapturing(null);
                  }}
                  onClick={() => setCapturing(s.id)}
                >
                  <span className="mono">{capturing === s.id ? "Pressione…" : s.keys}</span>
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Dialog>
  );
}
