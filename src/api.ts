import { invoke } from "@tauri-apps/api/core";
import type { AuditResult, DeviceStart, OwnerInfo, RepoInfo, RepoSyncResult, SettingChange, TeamInfo, UserInfo } from "./types";

export const authWithGh = () => invoke<UserInfo>("auth_with_gh");
export const authWithPat = (pat: string, save: boolean) => invoke<UserInfo>("auth_with_pat", { pat, save });
export const authLoadSaved = () => invoke<UserInfo | null>("auth_load_saved");
export const authDeviceStart = (clientId: string) => invoke<DeviceStart>("auth_device_start", { clientId });
export const authDevicePoll = (clientId: string, deviceCode: string) =>
  invoke<UserInfo | null>("auth_device_poll", { clientId, deviceCode });
export const logout = () => invoke<void>("logout");
export const listRepos = () => invoke<RepoInfo[]>("list_repos");
export const listOwners = () => invoke<OwnerInfo[]>("list_owners");
export const listReposForOwner = (owner: string, isOrg: boolean) =>
  invoke<RepoInfo[]>("list_repos_for_owner", { owner, isOrg });
export const listOrgTeams = (org: string) => invoke<TeamInfo[]>("list_org_teams", { org });
export const listTeamRepos = (org: string, teamSlug: string) =>
  invoke<string[]>("list_team_repos", { org, teamSlug });
export const audit = (reference: string, targets: string[]) => invoke<AuditResult>("audit", { reference, targets });
export const applySync = (plans: { repo: string; changes: SettingChange[] }[]) =>
  invoke<RepoSyncResult[]>("apply_sync", { plans });
