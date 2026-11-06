// Design System: componentes visuais sem regra de negócio, reutilizáveis em qualquer contexto.
import { useCallback, useEffect, useId, useRef, useState, type ButtonHTMLAttributes, type ReactNode, type Ref } from "react";

type BtnVariant = "default" | "primary" | "ghost" | "danger";

export function Button({ variant = "default", size, icon, className = "", ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: BtnVariant; size?: "sm"; icon?: boolean; ref?: Ref<HTMLButtonElement> }) {
  const cls = ["btn", variant !== "default" && `btn--${variant}`, size && `btn--${size}`, icon && "btn--icon", className].filter(Boolean).join(" ");
  return <button type="button" className={cls} {...rest} />;
}

export function Toggle({ pressed, onChange, children, className = "", ...rest }: Omit<ButtonHTMLAttributes<HTMLButtonElement>, "onChange"> & { pressed: boolean; onChange: (v: boolean) => void }) {
  return (
    <button type="button" className={`btn btn--sm ${className}`} aria-pressed={pressed} onClick={() => onChange(!pressed)} {...rest}>
      {children}
    </button>
  );
}

export function Field({ label, children, error }: { label: string; children: ReactNode; error?: string | null }) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
      {error ? <em className="field-error">{error}</em> : null}
    </label>
  );
}

export function Segmented<T extends string | number>({ value, options, onChange, label }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void; label: string }) {
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button key={String(o.value)} type="button" role="radio" aria-checked={o.value === value} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Tabs<T extends string>({ value, tabs, onChange, label }: { value: T; tabs: { id: T; label: string }[]; onChange: (v: T) => void; label: string }) {
  return (
    <div className="tabs" role="tablist" aria-label={label}>
      {tabs.map((t) => (
        <button key={t.id} role="tab" aria-selected={t.id === value} onClick={() => onChange(t.id)}>
          {t.label}
        </button>
      ))}
    </div>
  );
}

export function Dialog({ title, open, onClose, children, footer, size }: { title: string; open: boolean; onClose: () => void; children: ReactNode; footer?: ReactNode; size?: "lg" }) {
  const id = useId();
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const prev = document.activeElement as HTMLElement | null;
    ref.current?.querySelector<HTMLElement>("input, select, button, [tabindex]")?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    const dialogEl = ref.current;
    return () => {
      window.removeEventListener("keydown", onKey, true);
      // devolve o foco só se ninguém o assumiu depois (ex.: o editor que o diálogo acabou de abrir)
      const active = document.activeElement;
      const unclaimed = !active || active === document.body || (dialogEl?.contains(active) ?? false);
      if (unclaimed) prev?.focus();
    };
  }, [open, onClose]);
  if (!open) return null;
  return (
    <div className="dialog-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className={size === "lg" ? "dialog dialog--lg" : "dialog"} role="dialog" aria-modal="true" aria-labelledby={id} ref={ref}>
        <header id={id}>{title}</header>
        <div className="dialog-body">{children}</div>
        {footer ? <footer>{footer}</footer> : null}
      </div>
    </div>
  );
}

/**
 * Texto que vira campo ao clicar (ou Enter/F2 com foco). Enter ou perder o foco confirma; Esc cancela.
 * Só chama `onCommit` quando o valor (sem espaços nas pontas) muda e não fica vazio.
 */
export function InlineRename({ value, onCommit, label, inputLabel, className = "", children }: { value: string; onCommit: (v: string) => void; label: string; inputLabel: string; className?: string; children?: ReactNode }) {
  const [editing, setEditing] = useState(false);
  const cancelled = useRef(false);
  if (editing) {
    return (
      <input
        className={`input inline-rename-input ${className}`}
        autoFocus
        defaultValue={value}
        aria-label={inputLabel}
        onFocus={(e) => e.currentTarget.select()}
        onBlur={(e) => {
          setEditing(false);
          const v = e.currentTarget.value.trim();
          if (!cancelled.current && v && v !== value) onCommit(v);
          cancelled.current = false;
        }}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === "Enter") e.currentTarget.blur();
          if (e.key === "Escape") {
            cancelled.current = true;
            e.currentTarget.blur();
          }
        }}
      />
    );
  }
  return (
    <button
      type="button"
      className={`inline-rename ${className}`}
      aria-label={label}
      title={label}
      onClick={() => setEditing(true)}
      onKeyDown={(e) => {
        if (e.key === "F2") setEditing(true);
      }}
    >
      {children ?? value}
    </button>
  );
}

/** Estado mínimo de um histórico de desfazer (sessão, biblioteca…), sem depender do IPC. */
export interface UndoState {
  canUndo: boolean;
  canRedo: boolean;
  undoLabel?: string | null;
  redoLabel?: string | null;
}

/** Par de botões Desfazer/Refazer para um histórico; `scope` entra no rótulo acessível. */
export function UndoRedoButtons({ history, onUndo, onRedo, scope }: { history: UndoState | null; onUndo: () => void; onRedo: () => void; scope: string }) {
  return (
    <>
      <Button size="sm" disabled={!history?.canUndo} title={history?.undoLabel ?? undefined} onClick={onUndo} aria-label={`Desfazer ${scope}`}>
        ↶ {history?.undoLabel ? `Desfazer: ${history.undoLabel}` : "Desfazer"}
      </Button>
      <Button size="sm" disabled={!history?.canRedo} title={history?.redoLabel ?? undefined} onClick={onRedo} aria-label={`Refazer ${scope}`}>
        ↷ Refazer
      </Button>
    </>
  );
}

export function EmptyState({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <div className="empty">
      <div>
        <strong>{title}</strong>
        {children ? <div style={{ marginTop: 8 }}>{children}</div> : null}
      </div>
    </div>
  );
}

/** Mapeia dB ↔ posição 0..1 do fader (curva usada por DAWs: mais resolução perto de 0 dB). */
export function dbToFader(db: number): number {
  if (db <= -60) return Math.max(0, (db + 144) / 84) * 0.05;
  if (db <= 0) return 0.05 + ((db + 60) / 60) ** 1.6 * 0.75;
  return 0.8 + (Math.min(db, 12) / 12) * 0.2;
}

export function faderToDb(p: number): number {
  const x = Math.min(1, Math.max(0, p));
  if (x <= 0.05) return x / 0.05 * 84 - 144;
  if (x <= 0.8) return ((x - 0.05) / 0.75) ** (1 / 1.6) * 60 - 60;
  return ((x - 0.8) / 0.2) * 12;
}

/** Arrasto vertical genérico com Shift para ajuste fino. */
function useVerticalDrag(value: number, onChange: (v: number, end: boolean) => void, sensitivity: number) {
  return useCallback(
    (e: React.PointerEvent) => {
      const el = e.currentTarget as HTMLElement;
      el.setPointerCapture(e.pointerId);
      const startY = e.clientY;
      const start = value;
      let last = start;
      const move = (ev: PointerEvent) => {
        const k = ev.shiftKey ? sensitivity / 8 : sensitivity;
        last = start + (startY - ev.clientY) * k;
        onChange(last, false);
      };
      const up = () => {
        el.removeEventListener("pointermove", move);
        el.removeEventListener("pointerup", up);
        onChange(last, true);
      };
      el.addEventListener("pointermove", move);
      el.addEventListener("pointerup", up);
    },
    [value, onChange, sensitivity],
  );
}

/** Mantém o último valor enviado até o núcleo confirmar (evita perder ajustes rápidos).
 * Retorna [valor para desenhar, ref sempre atual, setter]. */
function useOptimistic(value: number): [number, React.MutableRefObject<number>, (v: number) => void] {
  const [local, setLocal] = useState(value);
  const ref = useRef(value);
  useEffect(() => {
    ref.current = value;
    setLocal(value);
  }, [value]);
  const set = useCallback((v: number) => {
    ref.current = v;
    setLocal(v);
  }, []);
  return [local, ref, set];
}

export function Fader({ db: confirmedDb, onChange: emit, label, height = 160 }: { db: number; onChange: (db: number, end: boolean) => void; label: string; height?: number }) {
  const [db, current, setLocal] = useOptimistic(confirmedDb);
  const onChange = useCallback((v: number, end: boolean) => {
    setLocal(v);
    emit(v, end);
  }, [emit, setLocal]);
  const pos = dbToFader(db);
  const onPos = useCallback((p: number, end: boolean) => onChange(Math.round(faderToDb(p) * 10) / 10, end), [onChange]);
  const onPointerDown = useVerticalDrag(pos, onPos, 1 / (height - 12));
  const usable = height - 12;
  return (
    <div
      className="fader"
      style={{ height }}
      role="slider"
      tabIndex={0}
      aria-label={label}
      aria-valuemin={-144}
      aria-valuemax={12}
      aria-valuenow={db}
      aria-valuetext={db <= -144 ? "-∞ dB" : `${db.toFixed(1)} dB`}
      onPointerDown={onPointerDown}
      onDoubleClick={() => onChange(0, true)}
      onKeyDown={(e) => {
        const step = e.shiftKey ? 0.1 : 1;
        const now = current.current;
        if (e.key === "ArrowUp") onChange(Math.round(Math.min(12, now + step) * 10) / 10, true);
        else if (e.key === "ArrowDown") onChange(Math.round(Math.max(-144, now - step) * 10) / 10, true);
        else return;
        e.preventDefault();
      }}
    >
      <div className="fader-track" />
      <div className="fader-unity" style={{ top: 6 + (1 - dbToFader(0)) * usable }} />
      <div className="fader-thumb" style={{ top: 6 + (1 - pos) * usable }} />
    </div>
  );
}

export function Knob({ value: confirmed, min, max, onChange: emit, label, format }: { value: number; min: number; max: number; onChange: (v: number, end: boolean) => void; label: string; format?: (v: number) => string }) {
  const [value, current, setLocal] = useOptimistic(confirmed);
  const onChange = (v: number, end: boolean) => {
    setLocal(v);
    emit(v, end);
  };
  const norm = (value - min) / (max - min);
  const onPointerDown = useVerticalDrag(value, (v, end) => onChange(Math.min(max, Math.max(min, v)), end), (max - min) / 150);
  return (
    <div
      className="knob"
      role="slider"
      tabIndex={0}
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      aria-valuetext={format ? format(value) : String(value)}
      title={`${label}: ${format ? format(value) : value}`}
      onPointerDown={onPointerDown}
      onDoubleClick={() => onChange((min + max) / 2, true)}
      onKeyDown={(e) => {
        const step = (max - min) / (e.shiftKey ? 200 : 40);
        const now = current.current;
        if (e.key === "ArrowUp" || e.key === "ArrowRight") onChange(Math.min(max, now + step), true);
        else if (e.key === "ArrowDown" || e.key === "ArrowLeft") onChange(Math.max(min, now - step), true);
        else return;
        e.preventDefault();
      }}
    >
      <div className="knob-indicator" style={{ transform: `rotate(${-135 + norm * 270}deg)` }} />
    </div>
  );
}

/** Medidor de pico estéreo; `peak` linear (0..1+). */
export function Meter({ peak, height = 160, label }: { peak: [number, number]; height?: number; label: string }) {
  const h = (x: number) => {
    const db = x > 0 ? 20 * Math.log10(x) : -144;
    return Math.max(0, Math.min(1, (db + 60) / 60));
  };
  return (
    <div className="meter" style={{ height, ["--h" as string]: `${height}px` }} role="meter" aria-label={label} aria-valuemin={0} aria-valuemax={1} aria-valuenow={Math.max(...peak)}>
      {peak.map((p, i) => (
        <div className="meter-bar" key={i}>
          <div className="meter-fill" style={{ height: `${h(p) * 100}%` }} />
          {p >= 1 ? <div className="meter-clip" /> : null}
        </div>
      ))}
    </div>
  );
}
