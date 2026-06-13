export type Category = "default_branch" | "features" | "pull_requests" | "others" | "tags" | "rules";

export interface UserInfo { login: string; avatar_url: string; scope_warning: string | null; }

export interface RepoInfo {
  full_name: string;
  owner: string;
  name: string;
  private: boolean;
  admin: boolean;
  archived: boolean;
  default_branch: string;
  description: string | null;
}

export interface SettingChange {
  key: string;
  label: string;
  category: Category;
  current: unknown;
  desired: unknown;
  applicable: boolean;
  note: string | null;
}

export interface RepoDiff { repo: string; changes: SettingChange[]; }

export interface AuditResult {
  reference: unknown;
  diffs: RepoDiff[];
  errors: [string, string][];
  // Campos de UI (no provienen del backend): estado del streaming.
  streaming?: boolean;
  total?: number;
}

// Eventos emitidos por el comando `audit` durante el streaming.
export interface AuditStartedEvent { total: number; }
export interface AuditRepoEvent { repo: string; diff: RepoDiff | null; error: string | null; }

export interface TeamInfo { slug: string; name: string; }
export interface ActionResult { description: string; ok: boolean; error: string | null; }
export interface RepoSyncResult { repo: string; results: ActionResult[]; fatal: string | null; }
export interface DeviceStart { device_code: string; user_code: string; verification_uri: string; interval: number; }

export const CATEGORY_LABELS: Record<Category, string> = {
  default_branch: "Default branch",
  features: "Features",
  pull_requests: "Pull Requests",
  others: "Others",
  tags: "Tags",
  rules: "Rules",
};

export const CATEGORY_ORDER: Category[] = ["default_branch", "features", "pull_requests", "others", "tags", "rules"];
