import { api } from "../../ipc/api";
import { run, useSession } from "../../core/session";
import { Button } from "../../shared/components";

/** Sobe/desce a faixa na ordem do arranjo (e do mixer); acessível por teclado (Tab + Enter/Espaço). */
export function TrackOrderButtons({ trackId, name, index }: { trackId: string; name: string; index: number }) {
  const count = useSession((s) => s.session?.tracks.length ?? 0);
  const move = (to: number) => void run(() => api.trackMove(trackId, to));
  return (
    <>
      <Button size="sm" variant="ghost" icon className="track-order" aria-label={`Mover ${name} para cima`} title="Mover para cima" disabled={index <= 0} onClick={() => move(index - 1)}>
        ▲
      </Button>
      <Button size="sm" variant="ghost" icon className="track-order" aria-label={`Mover ${name} para baixo`} title="Mover para baixo" disabled={index >= count - 1} onClick={() => move(index + 1)}>
        ▼
      </Button>
    </>
  );
}
