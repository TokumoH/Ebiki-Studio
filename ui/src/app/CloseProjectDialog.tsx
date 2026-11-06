import { api } from "../ipc/api";
import { run, useSession } from "../core/session";
import { Button, Dialog } from "../shared/components";

/** Fecha o projeto aberto; devolve `true` se o núcleo o fechou. */
export async function closeProject(): Promise<boolean> {
  const ok = await run(() => api.projectClose());
  if (ok) useSession.getState().setSession(null);
  return ok;
}

/** Confirmação ao fechar com alterações não salvas. O diálogo só some quando a ação dá certo. */
export function CloseProjectDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const discard = async () => {
    if (await closeProject()) onClose();
  };
  const saveAndClose = async () => {
    // falha ao salvar (ou ao fechar): mantém o diálogo aberto; o erro já aparece como aviso
    if ((await run(() => api.projectSave())) && (await closeProject())) onClose();
  };
  return (
    <Dialog
      title="Fechar sem salvar?"
      open={open}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancelar</Button>
          <Button variant="danger" onClick={() => void discard()}>
            Fechar sem salvar
          </Button>
          <Button variant="primary" onClick={() => void saveAndClose()}>
            Salvar e fechar
          </Button>
        </>
      }
    >
      <p>O projeto tem alterações que ainda não foram salvas.</p>
    </Dialog>
  );
}
