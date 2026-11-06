// Diálogo de atualização (mockup "6 · Atualizações"): nova versão, download com progresso,
// "já está na mais recente" e erros (sem conexão, assinatura recusada).
import type { ReactNode } from "react";
import { Button, Dialog } from "../shared/components";
import { APP_VERSION } from "./about";
import { formatProgress, useUpdates } from "./updates";

/**
 * Texto simples das novidades a partir das notas da release (Markdown do CHANGELOG):
 * usa só a seção "Novidades" quando existir e remove a marcação. Nunca interpreta HTML.
 */
export function plainNotes(md: string): string {
  const section = /^###\s+Novidades\s*$([\s\S]*?)(?=^###?\s|(?![\s\S]))/m.exec(md);
  const body = section ? section[1]! : md;
  return body
    .split("\n")
    .map((l) =>
      l
        .replace(/^#{1,6}\s*/, "")
        .replace(/^\s*[-*]\s+/, "• ")
        .replace(/\*\*(.+?)\*\*/g, "$1")
        .replace(/`([^`]+)`/g, "$1")
        .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1"),
    )
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

const formatDate = (iso?: string) => {
  if (!iso) return null;
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? null : d.toLocaleDateString("pt-BR", { day: "numeric", month: "long", year: "numeric" });
};

const ERROR_TEXT = {
  offline: { title: "Não foi possível verificar", body: "Sem conexão com a internet. O app continua funcionando normalmente offline." },
  signature: { title: "Atualização recusada", body: "A assinatura do arquivo não confere. Nada foi instalado." },
  other: { title: "Não foi possível atualizar", body: "O servidor de atualizações não respondeu como esperado. Tente mais tarde ou baixe pela página de releases." },
} as const;

export function UpdateDialog({ beforeRestart }: { beforeRestart: () => Promise<void> }) {
  const u = useUpdates();
  const close = u.later;
  const busy = u.status === "downloading" || u.status === "restarting";
  const checkbox = (
    <label className="update-auto">
      <input type="checkbox" checked={u.prefs.checkOnOpen} onChange={(e) => u.setCheckOnOpen(e.target.checked)} /> Verificar ao abrir o app
    </label>
  );

  let body: ReactNode = null;
  let footer: ReactNode = <Button onClick={close}>Fechar</Button>;

  if (u.status === "checking" || u.status === "idle") {
    body = <p className="update-line" role="status">Procurando atualizações…</p>;
  } else if (u.status === "upToDate") {
    body = (
      <div className="update-state" role="status">
        <span className="update-icon ok" aria-hidden="true">✓</span>
        <div>
          <strong>Você está na versão mais recente</strong>
          <span>Ebiki Studio {APP_VERSION}</span>
        </div>
      </div>
    );
  } else if (u.status === "error") {
    const t = ERROR_TEXT[u.error ?? "other"];
    body = (
      <div className="update-state" role="alert">
        <span className={`update-icon ${u.error === "signature" ? "bad" : "warn"}`} aria-hidden="true">
          {u.error === "signature" ? "✕" : "!"}
        </span>
        <div>
          <strong>{t.title}</strong>
          <span>{t.body}</span>
        </div>
      </div>
    );
    footer = (
      <>
        {u.error !== "signature" ? <Button onClick={() => void u.check({ manual: true })}>Tentar de novo</Button> : null}
        <Button onClick={close}>Fechar</Button>
      </>
    );
  } else if (u.info) {
    const date = formatDate(u.info.date);
    const notes = plainNotes(u.info.notes);
    body = (
      <div className="update">
        <div className="update-head">
          <span className="brand-mark update-logo" role="img" aria-label="Ebiki Studio" />
          <div>
            <strong>Atualização disponível</strong>
            <span>
              Você tem a <b>{APP_VERSION}</b> · nova: <b className="update-new">{u.info.version}</b>
              {date ? ` (${date})` : ""}
            </span>
          </div>
        </div>
        {notes ? (
          <div className="update-notes" tabIndex={0} aria-label="Novidades">
            <strong>Novidades</strong>
            <p>{notes}</p>
          </div>
        ) : null}
        {busy ? (
          <div className="update-progress" role="status">
            <span>{u.status === "restarting" ? "Reiniciando…" : `Baixando ${u.info.version}…`}</span>
            <progress max={u.progress.total ?? undefined} value={u.progress.total ? u.progress.done : undefined} aria-label="Progresso do download" />
            <span className="update-muted">{formatProgress(u.progress.done, u.progress.total)} · depois verifica a assinatura e reinicia</span>
          </div>
        ) : u.mode === "inApp" ? (
          <span className="update-muted">
            <span className="update-ok" aria-hidden="true">✓</span> Download assinado e verificado antes de instalar. Alterações não salvas são salvas antes de reiniciar.
          </span>
        ) : (
          <span className="update-muted">Esta instalação (.deb/.rpm) é atualizada pelo instalador da nova versão, na página de downloads.</span>
        )}
      </div>
    );
    footer = busy ? null : (
      <>
        <button type="button" className="update-skip" onClick={u.skip}>
          Pular esta versão
        </button>
        <Button onClick={close}>Depois</Button>
        {u.mode === "inApp" ? (
          <Button variant="primary" onClick={() => void u.install(beforeRestart)}>
            Instalar e reiniciar
          </Button>
        ) : (
          <Button variant="primary" onClick={() => void u.openPage()}>
            Abrir página de download
          </Button>
        )}
      </>
    );
  }

  return (
    <Dialog
      title="Atualizações"
      open={u.open}
      onClose={busy ? () => {} : close}
      footer={
        <div className="update-footer">
          {checkbox}
          <span style={{ flex: 1 }} />
          {footer}
        </div>
      }
    >
      {body}
    </Dialog>
  );
}
