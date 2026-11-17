// Avisos temporários (sucesso/erro) no canto da tela. Erros são anunciados como alerta (leitor de tela
// interrompe — UX-21); sucesso e informação, de forma educada. Aviso pode ter um botão de ação.
import { useToasts, type Toast } from "../core/toasts";
import { Button } from "../shared/components";

function Item({ t, dismiss }: { t: Toast; dismiss: (id: number) => void }) {
  return (
    <div className={`toast toast--${t.kind}`}>
      <p>{t.message}</p>
      {t.action ? (
        <Button
          size="sm"
          onClick={() => {
            dismiss(t.id);
            t.action!.run();
          }}
        >
          {t.action.label}
        </Button>
      ) : null}
      <Button size="sm" variant="ghost" icon aria-label="Fechar aviso" onClick={() => dismiss(t.id)}>
        ✕
      </Button>
    </div>
  );
}

export function Toasts() {
  const { toasts, dismiss } = useToasts();
  const errors = toasts.filter((t) => t.kind === "error");
  const others = toasts.filter((t) => t.kind !== "error");
  return (
    <div className="toasts">
      <div className="toasts-group" role="alert" aria-live="assertive">
        {errors.map((t) => (
          <Item key={t.id} t={t} dismiss={dismiss} />
        ))}
      </div>
      <div className="toasts-group" role="status" aria-live="polite">
        {others.map((t) => (
          <Item key={t.id} t={t} dismiss={dismiss} />
        ))}
      </div>
    </div>
  );
}
