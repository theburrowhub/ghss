interface Props {
  error: string | null;
  warning: string | null;
  status: string;
  onDismiss: () => void;
  onDismissWarning: () => void;
}

/** Translates raw backend errors into readable, actionable messages. */
export function friendlyError(raw: string): string {
  // Patterns that don't depend on JSON.
  if (/\b401\b/.test(raw) || raw.includes("invalid session")) {
    return "Your GitHub session is invalid or expired. Please reconnect.";
  }
  if (raw.toLowerCase().includes("rate limit")) {
    return "You've hit GitHub's rate limit. Wait a few minutes and try again.";
  }
  // Pass through gh binary not-found errors as-is (already English from backend).
  if (raw.includes("gh") && raw.includes("not found")) return raw;

  const brace = raw.indexOf("{");
  if (brace >= 0) {
    try {
      const obj = JSON.parse(raw.slice(brace));
      if (obj && typeof obj.message === "string") {
        const msg: string = obj.message;
        if (msg.toLowerCase().includes("rate limit")) {
          return "You've hit GitHub's rate limit. Wait a few minutes and try again.";
        }
        if (msg.includes("Upgrade to GitHub Pro")) {
          return "This feature (rulesets/protection) requires a public repo or a paid plan. It's skipped on those repos.";
        }
        const status = raw.match(/\b(\d{3})\b/)?.[1] ?? obj.status;
        if (status === "403" || status === 403) return `No permissions (403): ${msg}`;
        return status ? `GitHub ${status}: ${msg}` : msg;
      }
    } catch {
      /* not JSON: return text as-is */
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
        <button className="status-dismiss" onClick={onDismiss}>Dismiss</button>
      </div>
    );
  }
  if (warning) {
    return (
      <div className="statusbar warning">
        <span aria-hidden>⚠</span>
        <span className="status-text">{warning}</span>
        <div style={{ flex: 1 }} />
        <button className="status-dismiss" onClick={onDismissWarning}>Dismiss</button>
      </div>
    );
  }
  return (
    <div className="statusbar">
      <span className="status-text muted">{status || "Ready"}</span>
    </div>
  );
}
