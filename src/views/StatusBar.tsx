interface Props {
  error: string | null;
  status: string;
  onDismiss: () => void;
}

/** Traduce los errores crudos del backend a algo legible (extrae el message de la API de GitHub). */
export function friendlyError(raw: string): string {
  const brace = raw.indexOf("{");
  if (brace >= 0) {
    try {
      const obj = JSON.parse(raw.slice(brace));
      if (obj && typeof obj.message === "string") {
        const status = raw.match(/respondió\s+(\d{3})/)?.[1] ?? obj.status;
        return status ? `GitHub ${status}: ${obj.message}` : obj.message;
      }
    } catch {
      /* no era JSON: devolvemos el texto tal cual */
    }
  }
  return raw;
}

export function StatusBar({ error, status, onDismiss }: Props) {
  return (
    <div className={`statusbar${error ? " error" : ""}`}>
      {error ? (
        <>
          <span aria-hidden>⚠</span>
          <span className="status-text">{friendlyError(error)}</span>
          <div style={{ flex: 1 }} />
          <button className="status-dismiss" onClick={onDismiss}>Descartar</button>
        </>
      ) : (
        <span className="status-text muted">{status || "Listo"}</span>
      )}
    </div>
  );
}
