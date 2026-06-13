import { useCallback, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { applySync, audit, listRepos, logout } from "./api";
import type { AuditRepoEvent, AuditResult, AuditStartedEvent, RepoInfo, RepoSyncResult, SettingChange, UserInfo } from "./types";
import { AuthView } from "./views/AuthView";
import { ReposView } from "./views/ReposView";
import { AuditView } from "./views/AuditView";
import { ExecutionView } from "./views/ExecutionView";
import { LoadingView } from "./views/LoadingView";
import { StatusBar } from "./views/StatusBar";

type Stage = "auth" | "loading" | "repos" | "audit" | "exec";

export default function App() {
  const [stage, setStage] = useState<Stage>("auth");
  const [user, setUser] = useState<UserInfo | null>(null);
  const [repos, setRepos] = useState<RepoInfo[]>([]);
  const [reference, setReference] = useState<string | null>(null);
  const [targets, setTargets] = useState<Set<string>>(new Set());
  const [auditResult, setAuditResult] = useState<AuditResult | null>(null);
  const [syncResults, setSyncResults] = useState<RepoSyncResult[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const [loading, setLoading] = useState<{ title: string; detail?: string }>({ title: "" });

  const onLogin = useCallback(async (u: UserInfo) => {
    setUser(u);
    setError(null);
    setLoading({ title: "Cargando repositorios…", detail: `Conectado como ${u.login}. Obteniendo la lista de repos accesibles.` });
    setStage("loading");
    try {
      const list = await listRepos();
      setRepos(list);
      setStage("repos");
    } catch (e) {
      setError(String(e));
      setStage("auth");
    }
  }, []);

  const runAudit = async () => {
    if (!reference) return;
    setError(null);
    const targetList = Array.from(targets).filter((t) => t !== reference);
    setAuditResult({ reference: null, diffs: [], errors: [], streaming: true, total: targetList.length });
    setStage("audit");

    const accumulate = (payload: AuditRepoEvent) =>
      setAuditResult((prev) => {
        if (!prev) return prev;
        if (payload.diff) return { ...prev, diffs: [...prev.diffs, payload.diff] };
        if (payload.error) return { ...prev, errors: [...prev.errors, [payload.repo, payload.error]] };
        return prev;
      });

    const unStarted = await listen<AuditStartedEvent>("audit-started", (e) =>
      setAuditResult((prev) => (prev ? { ...prev, total: e.payload.total } : prev)),
    );
    const unRepo = await listen<AuditRepoEvent>("audit-repo", (e) => accumulate(e.payload));

    try {
      const final = await audit(reference, targetList);
      setAuditResult({ ...final, streaming: false, total: targetList.length });
    } catch (e) {
      setError(String(e));
      setStage("repos");
    } finally {
      unStarted();
      unRepo();
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
      setStage("audit");
    } finally {
      setBusy(false);
    }
  };

  const doLogout = async () => {
    await logout().catch(() => {});
    setUser(null);
    setStatus("");
    setStage("auth");
  };

  return (
    <>
      <div className="topbar">
        <strong>ghss</strong>
        <span className="muted">GitHub Settings Sync</span>
        <div className="spacer" />
        {user && (
          <>
            <img className="avatar" src={user.avatar_url} alt="" />
            <span>{user.login}</span>
            <button onClick={doLogout}>Salir</button>
          </>
        )}
      </div>

      {stage === "auth" && <AuthView onLogin={onLogin} />}
      {stage === "loading" && <LoadingView title={loading.title} detail={loading.detail} />}
      {stage === "repos" && (
        <ReposView
          repos={repos}
          reference={reference}
          targets={targets}
          onReference={setReference}
          onTargets={setTargets}
          onAudit={runAudit}
          onStatus={setStatus}
          busy={busy}
        />
      )}
      {stage === "audit" && reference && auditResult && (
        <AuditView
          reference={reference}
          result={auditResult}
          onBack={() => setStage("repos")}
          onSync={runSync}
          onStatus={setStatus}
          busy={busy}
        />
      )}
      {stage === "exec" && (
        <ExecutionView
          results={syncResults}
          onDone={async () => { setSyncResults(null); setAuditResult(null); setStage("repos"); }}
        />
      )}

      <StatusBar error={error} status={status} onDismiss={() => setError(null)} />
    </>
  );
}
