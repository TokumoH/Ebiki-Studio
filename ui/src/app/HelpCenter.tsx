// Central de ajuda (F1): busca, tópicos do manual, atalhos de teclado e acesso ao tutorial.
import { useMemo, useState } from "react";
import { Button, Dialog } from "../shared/components";
import { HELP_TOPICS, searchTopics, type HelpTopic } from "./helpTopics";
import { loadShortcuts } from "./shortcuts";
import { APP_VERSION } from "./about";

const SHORTCUTS_ID = "atalhos";

type Entry = HelpTopic & { shortcuts?: boolean };

function topics(): Entry[] {
  const sc = loadShortcuts();
  return [
    ...HELP_TOPICS,
    {
      id: SHORTCUTS_ID,
      title: "Atalhos de teclado",
      keywords: `teclado atalho tecla ${sc.map((s) => `${s.label} ${s.keys}`).join(" ")}`,
      intro: "Atalhos padrão de DAW. Edite em Ajuda › Atalhos de teclado (Ctrl+/).",
      steps: [],
      shortcuts: true,
    },
  ];
}

export function HelpCenter({ open, onClose, onStartTour, onEditShortcuts }: { open: boolean; onClose: () => void; onStartTour: (step?: number) => void; onEditShortcuts: () => void }) {
  const [query, setQuery] = useState("");
  const [current, setCurrent] = useState(HELP_TOPICS[0]!.id);
  const all = useMemo(() => (open ? topics() : []), [open]);
  const found = searchTopics(all, query);
  const topic = found.find((t) => t.id === current) ?? found[0];

  return (
    <Dialog title="Central de ajuda" open={open} onClose={onClose} size="lg">
      <div className="help">
        <nav className="help-nav" aria-label="Tópicos de ajuda">
          <input
            className="input"
            type="search"
            placeholder="Buscar: gravar, fade, exportar…"
            aria-label="Buscar na ajuda"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <ul className="help-topics">
            {found.map((t) => (
              <li key={t.id}>
                <button type="button" className="help-topic" aria-current={t.id === topic?.id ? "page" : undefined} onClick={() => setCurrent(t.id)}>
                  {t.title}
                </button>
              </li>
            ))}
          </ul>
          {found.length === 0 ? <p className="help-empty">Nada encontrado para “{query}”.</p> : null}
        </nav>
        <article className="help-article" aria-live="polite">
          {topic ? (
            <>
              <h2>{topic.title}</h2>
              <p className="help-intro">{topic.intro}</p>
              {topic.shortcuts ? (
                <table className="help-shortcuts">
                  <tbody>
                    {loadShortcuts().map((s) => (
                      <tr key={s.id}>
                        <td>{s.label}</td>
                        <td>
                          <kbd>{s.keys}</kbd>
                        </td>
                      </tr>
                    ))}
                    <tr>
                      <td>Central de ajuda</td>
                      <td>
                        <kbd>F1</kbd>
                      </td>
                    </tr>
                    <tr>
                      <td>Ir para o menu</td>
                      <td>
                        <kbd>Alt</kbd> ou <kbd>F10</kbd>
                      </td>
                    </tr>
                  </tbody>
                </table>
              ) : (
                <ol className="help-steps">
                  {topic.steps.map((s, i) => (
                    <li key={i}>{s}</li>
                  ))}
                </ol>
              )}
              <div className="row">
                {topic.tourStep ? (
                  <Button variant="primary" onClick={() => onStartTour(topic.tourStep)}>
                    Mostrar no app
                  </Button>
                ) : null}
                {topic.shortcuts ? <Button onClick={onEditShortcuts}>Editar atalhos</Button> : null}
              </div>
            </>
          ) : null}
          <div className="help-tour">
            <div>
              <strong>Primeira vez por aqui?</strong>
              <span>O tour mostra as áreas principais em 7 passos. Dá para pular a qualquer momento.</span>
            </div>
            <Button onClick={() => onStartTour(1)}>Refazer o tutorial</Button>
          </div>
          <p className="help-foot">Ebiki Studio {APP_VERSION} · offline · F1 abre esta ajuda</p>
        </article>
      </div>
    </Dialog>
  );
}
