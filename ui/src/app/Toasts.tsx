// Avisos temporários (sucesso/erro) no canto da tela.
import { useToasts } from "../core/toasts";
import { Button } from "../shared/components";

export function Toasts() {
  const { toasts, dismiss } = useToasts();
  return (
    <div className="toasts" role="status" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={`toast toast--${t.kind}`}>
          <p>{t.message}</p>
          <Button size="sm" variant="ghost" icon aria-label="Fechar aviso" onClick={() => dismiss(t.id)}>
            ✕
          </Button>
        </div>
      ))}
    </div>
  );
}
