import { useCallback, useState } from "react";
import { applySync, audit, listRepos, logout } from "./api";
import type { AuditResult, RepoInfo, RepoSyncResult, SettingChange, UserInfo } from "./types";
import { AuthView } from "./views/AuthView";
import { ReposView } from "./views/ReposView";
import { AuditView } from "./views/AuditView";
import { PreSyncView } from "./views/PreSyncView";
import { ExecutionView } from "./views/ExecutionView";

type Stage = "auth" | "repos" | "audit" | "presync" | "exec";

export default function App() {
  const [stage, setStage] = useState<Stage>("auth");
  const [user, setUser] = useState<UserInfo | null>(null);
  const [repos, setRepos] = useState<RepoInfo[]>([]);
  const [reference, setReference] = useState<string | null>(null);
  const [targets, setTargets] = useState<Set<string>>(new Set());
  const [auditResult, setAuditResult] = useState<AuditResult | null>(null);
  const [presyncRepos, setPresyncRepos] = useState<string[]>([]);
  const [syncResults, setSyncResults] = useState<RepoSyncResult[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const onLogin = useCallback(async (u: UserInfo) => {
    setUser(u);
    setBusy(true);
    try {
      setRepos(await listRepos());
      setStage("repos");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  const runAudit = async () => {
    if (!reference) return;
    setBusy(true);
    setError(null);
    try {
      // La referencia nunca se audita contra sí misma aunque siguiera marcada como destino.
      setAuditResult(await audit(reference, Array.from(targets).filter((t) => t !== reference)));
      setStage("audit");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const runSync = async (plans: { repo: string; changes: SettingChange[] }[]) => {
    setBusy(true);
    setError(null);
    setSyncResults(null);
    setStage("exec");
    try {
      setSyncResults(await applySync(plans));
    } catch (e) {
      setError(String(e));
      setStage("presync");
    } finally {
      setBusy(false);
    }
  };

  const doLogout = async () => {
    await logout().catch(() => {});
    setUser(null);
    setStage("auth");
  };

  return (
    <>
      <div className="topbar">
        <strong>ghss</strong>
        <span className="muted">GitHub Settings Sync</span>
        <div className="spacer" />
        {error && <span style={{ color: "var(--danger)" }}>{error}</span>}
        {user && (
          <>
            <img className="avatar" src={user.avatar_url} alt="" />
            <span>{user.login}</span>
            <button onClick={doLogout}>Salir</button>
          </>
        )}
      </div>

      {stage === "auth" && <AuthView onLogin={onLogin} />}
      {stage === "repos" && (
        <ReposView
          repos={repos}
          reference={reference}
          targets={targets}
          onReference={setReference}
          onTargets={setTargets}
          onAudit={runAudit}
          busy={busy}
        />
      )}
      {stage === "audit" && reference && auditResult && (
        <AuditView
          reference={reference}
          result={auditResult}
          onBack={() => setStage("repos")}
          onProceed={(repos) => { setPresyncRepos(repos); setStage("presync"); }}
        />
      )}
      {stage === "presync" && reference && auditResult && (
        <PreSyncView
          reference={reference}
          diffs={auditResult.diffs.filter((d) => presyncRepos.includes(d.repo) && d.changes.length > 0)}
          onBack={() => setStage("audit")}
          onSync={runSync}
          busy={busy}
        />
      )}
      {stage === "exec" && (
        <ExecutionView
          results={syncResults}
          onDone={async () => { setSyncResults(null); setAuditResult(null); setStage("repos"); }}
        />
      )}
    </>
  );
}
