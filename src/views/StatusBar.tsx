interface Props {
  error: string | null;
  warning: string | null;
  status: string;
  onDismiss: () => void;
  onDismissWarning: () => void;
}

/** Traduce los errores crudos del backend a algo legible y accionable. */
export function friendlyError(raw: string): string {
  // Patrones que no dependen del JSON.
  if (/\b401\b/.test(raw) || raw.includes("sesión no válida")) {
    return "Tu sesión de GitHub no es válida o caducó. Vuelve a conectar.";
  }
  if (raw.toLowerCase().includes("rate limit")) {
    return "Has alcanzado el límite de peticiones de GitHub. Espera unos minutos antes de reintentar.";
  }
  if (raw.includes("no se encontró el binario «gh»")) return raw;

  const brace = raw.indexOf("{");
  if (brace >= 0) {
    try {
      const obj = JSON.parse(raw.slice(brace));
      if (obj && typeof obj.message === "string") {
        const msg: string = obj.message;
        if (msg.toLowerCase().includes("rate limit")) {
          return "Has alcanzado el límite de peticiones de GitHub. Espera unos minutos antes de reintentar.";
        }
        if (msg.includes("Upgrade to GitHub Pro")) {
          return "Esta función (rulesets/protección) requiere repo público o plan de pago. Se ignora en esos repos.";
        }
        const status = raw.match(/respondió\s+(\d{3})/)?.[1] ?? obj.status;
        if (status === "403" || status === 403) return `Sin permisos (403): ${msg}`;
        return status ? `GitHub ${status}: ${msg}` : msg;
      }
    } catch {
      /* no era JSON: devolvemos el texto tal cual */
    }
  }
  return raw;
}

export function StatusBar({ error, warning, status, onDismiss, onDismissWarning }: Props) {
  if (error) {
    return (
      <div className="statusbar error">
        <span aria-hidden>⚠</span>
        <span className="status-text">{friendlyError(error)}</span>
        <div style={{ flex: 1 }} />
        <button className="status-dismiss" onClick={onDismiss}>Descartar</button>
      </div>
    );
  }
  if (warning) {
    return (
      <div className="statusbar warning">
        <span aria-hidden>⚠</span>
        <span className="status-text">{warning}</span>
        <div style={{ flex: 1 }} />
        <button className="status-dismiss" onClick={onDismissWarning}>Descartar</button>
      </div>
    );
  }
  return (
    <div className="statusbar">
      <span className="status-text muted">{status || "Listo"}</span>
    </div>
  );
}
