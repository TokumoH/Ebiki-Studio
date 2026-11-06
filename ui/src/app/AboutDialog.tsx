// Sobre: marca, versão, licença e privacidade em uma janela curta.
import { Button, Dialog } from "../shared/components";
import { APP_VERSION } from "./about";

export function AboutDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <Dialog title="Sobre o Ebiki Studio" open={open} onClose={onClose} footer={<Button onClick={onClose}>Fechar</Button>}>
      <div className="about">
        <span className="brand-mark about-logo" role="img" aria-label="Logo do Ebiki Studio" />
        <div className="about-text">
          <strong>Ebiki Studio {APP_VERSION}</strong>
          <span>Estúdio musical para gravar, mixar, editar, compor e afinar.</span>
          <span>Offline, sem contas e sem coleta de dados.</span>
          <span className="about-legal">Código sob GPL-3.0-or-later. O nome e a logo Ebiki Studio são reservados ao autor.</span>
        </div>
      </div>
    </Dialog>
  );
}
