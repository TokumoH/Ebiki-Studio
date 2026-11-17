// Diálogo de atalhos de teclado: lista e edita os atalhos (gravados no armazenamento local).
// Tecla já usada por outra ação: avisa e pergunta se substitui (a outra fica sem atalho). "Restaurar padrões"
// volta tudo ao original. As teclas aparecem em PT-BR (Espaço, setas; ⌘ no macOS).
import { useState } from "react";
import { Button, Dialog } from "../shared/components";
import { displayKeys } from "../shared/keys";
import { eventKeys, findConflict, loadShortcuts, resetShortcuts, saveShortcut, type Shortcut } from "./shortcuts";

export function ShortcutsDialog({ open, onClose, onChanged }: { open: boolean; onClose: () => void; onChanged: (s: Shortcut[]) => void }) {
  const [list, setList] = useState(loadShortcuts);
  const [capturing, setCapturing] = useState<string | null>(null);
  const [pending, setPending] = useState<{ id: string; keys: string; other: Shortcut } | null>(null);
  const commit = (next: Shortcut[]) => {
    setList(next);
    onChanged(next);
  };
  const assign = (id: string, keys: string, freeOther?: string) => {
    if (freeOther) saveShortcut(freeOther, "");
    saveShortcut(id, keys);
    commit(loadShortcuts());
    setPending(null);
    setCapturing(null);
  };
  return (
    <Dialog
      title="Atalhos de teclado"
      open={open}
      onClose={onClose}
      footer={
        <>
          <Button
            variant="ghost"
            onClick={() => {
              setPending(null);
              setCapturing(null);
              commit(resetShortcuts());
            }}
          >
            Restaurar padrões
          </Button>
          <Button onClick={onClose}>Fechar</Button>
        </>
      }
    >
      <table style={{ width: "100%", borderCollapse: "collapse" }}>
        <tbody>
          {list.map((s) => (
            <tr key={s.id}>
              <td style={{ padding: 4 }}>
                {s.label}
                {pending?.id === s.id ? (
                  <div className="shortcut-conflict" role="alert">
                    <span>
                      {displayKeys(pending.keys)} já é de “{pending.other.label}”.
                    </span>
                    <Button size="sm" variant="primary" onClick={() => assign(pending.id, pending.keys, pending.other.id)}>
                      Substituir
                    </Button>
                    <Button size="sm" onClick={() => setPending(null)}>
                      Cancelar
                    </Button>
                  </div>
                ) : null}
              </td>
              <td style={{ padding: 4, textAlign: "right", verticalAlign: "top" }}>
                <Button
                  size="sm"
                  aria-label={`Alterar atalho: ${s.label}`}
                  onKeyDown={(e) => {
                    if (capturing !== s.id) return;
                    e.preventDefault();
                    e.stopPropagation();
                    if (e.key === "Escape") return setCapturing(null);
                    const keys = eventKeys(e.nativeEvent);
                    if (["Ctrl", "Shift", "Alt", "Ctrl+Shift"].includes(keys)) return;
                    const other = findConflict(list, s.id, keys);
                    if (other) {
                      setPending({ id: s.id, keys, other });
                      setCapturing(null);
                      return;
                    }
                    assign(s.id, keys);
                  }}
                  onClick={() => setCapturing(s.id)}
                >
                  <span className="mono">{capturing === s.id ? "Pressione…" : s.keys ? displayKeys(s.keys) : "—"}</span>
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Dialog>
  );
}
