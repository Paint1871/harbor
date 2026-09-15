import { Fragment, useState, type ReactNode } from "react";
import type { Workspace } from "@harbor/schema/commands";
import { workspaceMatchesQuery } from "./workspaceMatchesQuery";

interface WorkspaceRailListProps {
  workspaces: Workspace[];
  empty?: ReactNode;
  children: (workspace: Workspace) => ReactNode;
}

export function WorkspaceRailList({ workspaces, empty, children }: WorkspaceRailListProps) {
  const [query, setQuery] = useState("");
  const visible = workspaces.filter((workspace) => workspaceMatchesQuery(workspace, query));
  const needle = query.trim();

  return (
    <>
      <label className="harbor-destination-search harbor-thread-search">
        Search workspaces
        <input
          aria-label="Search workspaces"
          value={query}
          placeholder="Find a workspace"
          onChange={(event) => setQuery(event.target.value)}
        />
      </label>
      {visible.length
        ? visible.map((workspace) => <Fragment key={workspace.id}>{children(workspace)}</Fragment>)
        : needle
          ? <p className="harbor-muted">No workspaces match “{query}”.</p>
          : empty}
    </>
  );
}
