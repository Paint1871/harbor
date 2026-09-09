import { invoke } from "@tauri-apps/api/core";
import type { CommandArgs, CommandResult, HarborCommand } from "@harbor/schema/commands";

/**
 * The only door to the host. Going through the contract means a misspelled
 * command, a missing argument or a renamed field fails `tsc` here instead of
 * rejecting at runtime in a built app — the failure mode `HarborCommands` was
 * written to prevent while nothing imported it.
 *
 * Commands that take no arguments declare `args: undefined` and are called with
 * the name alone.
 */
export function call<K extends HarborCommand>(
  command: K,
  ...args: CommandArgs<K> extends undefined ? [] : [CommandArgs<K>]
): Promise<CommandResult<K>> {
  // A command with no arguments is invoked with the name alone, exactly as a
  // direct `invoke` would send it.
  const sent = args.length ? invoke(command, args[0] as Record<string, unknown>) : invoke(command);
  return sent as Promise<CommandResult<K>>;
}
