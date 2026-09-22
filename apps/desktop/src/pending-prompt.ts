/**
 * Staged composer drafts live in settings under one prefix:
 * `pending_agent_prompt:<chatId>` prepares a specific chat (routines), while
 * `pending_agent_prompt:next` is picked up by the next agent composer that
 * mounts (skills). One mechanism, two scopes.
 */
export const PENDING_PROMPT_PREFIX = "pending_agent_prompt:";
export const PENDING_PROMPT_NEXT = `${PENDING_PROMPT_PREFIX}next`;

export function pendingPromptKey(chatId: string): string {
  return `${PENDING_PROMPT_PREFIX}${chatId}`;
}
