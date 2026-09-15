import type { Workspace } from "@harbor/schema/commands";
import { workspaceName } from "./WorkspaceRailRow";

export function workspaceMatchesQuery(
  workspace: Pick<Workspace, "title" | "folder">,
  query: string,
): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  if (workspaceName(workspace).toLowerCase().includes(needle)) return true;
  if ((workspace.title ?? "").toLowerCase().includes(needle)) return true;
  return workspace.folder.toLowerCase().includes(needle);
}
