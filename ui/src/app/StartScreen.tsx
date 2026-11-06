import { useEffect, useState } from "react";
import { api } from "../ipc/api";
import { transport } from "../ipc/transport";
import type { RecentProject } from "../ipc/types";
import { errorMessage, run } from "../core/session";
import { notify } from "../core/toasts";
import { useTheme } from "../core/theme";
import { Button, Field, Segmented, Dialog } from "../shared/components";

/** Tela inicial: novo projeto, abrir e recentes (com verificação de recuperação de autosave). */
export function StartScreen() {
  const [name, setName] = useState("Minha música");
  const [dir, setDir] = useState("");
  const [rate, setRate] = useState(48000);
  const [bits, setBits] = useState(24);
  const [recent, setRecent] = useState<RecentProject[]>([]);
  const [recoverPath, setRecoverPath] = useState<string | null>(null);
  const toggleTheme = useTheme((s) => s.toggle);
  const nameError = name.trim().length === 0 ? "Informe um nome." : [...name.trim()].length > 64 ? "No máximo 64 caracteres." : null;

  useEffect(() => {
    void api.projectRecent().then(setRecent).catch((e) => {
      setRecent([]);
      notify.error(`Não foi possível carregar os projetos recentes: ${errorMessage(e)}`);
    });
  }, []);

  const open = async (path: string) => {
    try {
      if (await api.projectRecoveryCheck(path)) {
        setRecoverPath(path);
        return;
      }
    } catch {
      /* segue para abrir normalmente */
    }
    const ok = await run(() => api.projectOpen(path), { busy: true });
    if (!ok) void api.projectRecent().then(setRecent).catch(() => undefined);
  };

  const pickDir = async () => {
    const r = await transport().pickOpen({ directory: true, title: "Pasta do projeto" });
    if (r?.[0]) setDir(r[0]);
  };

  const pickProject = async () => {
    const r = await transport().pickOpen({ directory: true, title: "Abrir projeto (.ebiki)" });
    if (r?.[0]) await open(r[0]);
  };

  return (
    <div className="start">
      <header>
        <div className="brand">
          <span className="brand-mark" role="img" aria-label="Ebiki Studio" /> Ebiki Studio
        </div>
        <Button variant="ghost" onClick={toggleTheme} aria-label="Alternar tema claro/escuro">
          ◐ Tema
        </Button>
      </header>
      <main>
        <section className="card" aria-labelledby="new-title">
          <h2 id="new-title">Novo projeto</h2>
          <form
            style={{ display: "flex", flexDirection: "column", gap: 12 }}
            onSubmit={(e) => {
              e.preventDefault();
              if (nameError || !dir) return;
              void run(() => api.projectCreate({ dir, name: name.trim(), sampleRate: rate, bitDepth: bits }), { busy: true });
            }}
          >
            <Field label="Nome" error={nameError}>
              <input className="input" value={name} onChange={(e) => setName(e.target.value)} aria-label="Nome do projeto" maxLength={80} />
            </Field>
            <Field label="Pasta">
              <div style={{ display: "flex", gap: 8 }}>
                <input className="input" value={dir} readOnly placeholder="Escolha onde salvar" aria-label="Pasta do projeto" style={{ flex: 1 }} />
                <Button onClick={() => void pickDir()}>Escolher…</Button>
              </div>
            </Field>
            <Field label="Taxa de amostragem">
              <Segmented label="Taxa de amostragem" value={rate} onChange={setRate} options={[44100, 48000, 88200, 96000].map((v) => ({ value: v, label: `${(v / 1000).toLocaleString("pt-BR")} kHz` }))} />
            </Field>
            <Field label="Resolução">
              <Segmented label="Resolução" value={bits} onChange={setBits} options={[{ value: 16, label: "16 bits" }, { value: 24, label: "24 bits" }, { value: 32, label: "32 float" }]} />
            </Field>
            <Button variant="primary" type="submit" disabled={!!nameError || !dir}>
              Criar projeto
            </Button>
          </form>
        </section>
        <section className="card" aria-labelledby="recent-title">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <h2 id="recent-title" style={{ margin: 0 }}>
              Recentes
            </h2>
            <Button onClick={() => void pickProject()}>Abrir projeto…</Button>
          </div>
          {recent.length === 0 ? (
            <p style={{ color: "var(--fg-muted)" }}>Nenhum projeto aberto recentemente.</p>
          ) : (
            <ul className="recent">
              {recent.map((r) => (
                <li key={r.path}>
                  <button className="open" onClick={() => void open(r.path)}>
                    <strong>{r.name}</strong>
                    <small>{r.path}</small>
                  </button>
                  <Button size="sm" variant="ghost" aria-label={`Remover ${r.name} dos recentes`} onClick={() => void api.projectForgetRecent(r.path).then(() => setRecent((x) => x.filter((y) => y.path !== r.path)))}>
                    ✕
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </section>
      </main>
      <Dialog
        title="Recuperar alterações?"
        open={recoverPath !== null}
        onClose={() => setRecoverPath(null)}
        footer={
          <>
            <Button
              onClick={() => {
                const p = recoverPath!;
                setRecoverPath(null);
                void api.projectDiscardRecovery(p).then(() => run(() => api.projectOpen(p), { busy: true }));
              }}
            >
              Abrir versão salva
            </Button>
            <Button
              variant="primary"
              onClick={() => {
                const p = recoverPath!;
                setRecoverPath(null);
                void run(() => api.projectRecover(p), { busy: true, success: "Alterações recuperadas. Salve para mantê-las." });
              }}
            >
              Recuperar
            </Button>
          </>
        }
      >
        <p>Existe um salvamento automático mais recente que o arquivo do projeto (o app pode ter fechado sem salvar).</p>
      </Dialog>
    </div>
  );
}
