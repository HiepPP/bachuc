/** Set by the client entry; surfaces call it to open the new workspace screen for a project. */
export const workspaceActions: {
  open:
    | ((input: { cwd: string; name?: string; projectId?: string; serverId?: string }) => void)
    | null;
} = { open: null };
