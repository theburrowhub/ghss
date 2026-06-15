import { useEffect, useRef, useState } from "react";
import { authDevicePoll, authDeviceStart, authLoadSaved, authWithGh, authWithPat } from "../api";
import { friendlyError } from "./StatusBar";
import type { DeviceStart, UserInfo } from "../types";

type Method = "gh" | "pat" | "device";

export function AuthView({ onLogin }: { onLogin: (u: UserInfo) => void }) {
  const [method, setMethod] = useState<Method>("gh");
  const [pat, setPat] = useState("");
  const [savePat, setSavePat] = useState(true);
  const [clientId, setClientId] = useState("");
  const [device, setDevice] = useState<DeviceStart | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const polling = useRef(false);

  useEffect(() => {
    authLoadSaved().then((u) => u && onLogin(u)).catch(() => {});
  }, [onLogin]);

  // Switching method stops any in-progress device flow polling.
  const switchMethod = (m: Method) => {
    polling.current = false;
    setMethod(m);
  };

  const run = async (fn: () => Promise<UserInfo>) => {
    setBusy(true);
    setError(null);
    try {
      onLogin(await fn());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const startDevice = async () => {
    setBusy(true);
    setError(null);
    try {
      const d = await authDeviceStart(clientId);
      setDevice(d);
      polling.current = true;
      while (polling.current) {
        await new Promise((r) => setTimeout(r, (d.interval + 1) * 1000));
        const u = await authDevicePoll(clientId, d.device_code);
        if (u) {
          polling.current = false;
          onLogin(u);
        }
      }
    } catch (e) {
      setError(String(e));
      polling.current = false;
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => () => { polling.current = false; }, []);

  return (
    <div className="view" style={{ display: "grid", placeItems: "center" }}>
      <div className="card" style={{ width: 440 }}>
        <h2 style={{ marginTop: 0 }}>Connect to GitHub</h2>
        <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
          <button onClick={() => switchMethod("gh")} className={method === "gh" ? "primary" : ""}>gh CLI</button>
          <button onClick={() => switchMethod("pat")} className={method === "pat" ? "primary" : ""}>Token (PAT)</button>
          <button onClick={() => switchMethod("device")} className={method === "device" ? "primary" : ""}>Device Flow</button>
        </div>

        {method === "gh" && (
          <>
            <p className="muted">Use your existing GitHub CLI session (<span className="mono">gh auth token</span>).</p>
            <button className="primary" disabled={busy} onClick={() => run(authWithGh)}>Connect with gh</button>
          </>
        )}

        {method === "pat" && (
          <>
            <p className="muted">Paste a Personal Access Token with repo admin permissions.</p>
            <input type="password" placeholder="ghp_… / github_pat_…" value={pat} onChange={(e) => setPat(e.target.value)} />
            <label style={{ display: "block", margin: "10px 0" }}>
              <input type="checkbox" checked={savePat} onChange={(e) => setSavePat(e.target.checked)} /> Save to the system keychain
            </label>
            <button className="primary" disabled={busy || !pat} onClick={() => run(() => authWithPat(pat, savePat))}>Connect</button>
          </>
        )}

        {method === "device" && (
          <>
            <p className="muted">Requires the Client ID of your own OAuth App (Settings → Developer settings).</p>
            <input type="text" placeholder="Client ID" value={clientId} onChange={(e) => setClientId(e.target.value)} />
            <div style={{ marginTop: 10 }}>
              <button className="primary" disabled={busy || !clientId} onClick={startDevice}>Start device flow</button>
            </div>
            {device && (
              <p>
                Enter code <strong className="mono">{device.user_code}</strong> at{" "}
                <a href={device.verification_uri} target="_blank" rel="noreferrer">{device.verification_uri}</a>
              </p>
            )}
          </>
        )}

        {error && <p style={{ color: "var(--danger)" }}>{friendlyError(error)}</p>}
      </div>
    </div>
  );
}
