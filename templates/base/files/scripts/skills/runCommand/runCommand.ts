import { spawn } from "node:child_process";
import type { CommandResult, CommandSpec } from "../skills.types";

/**
 * Runs a command and resolves with its result.
 *
 * A failure resolves with a non-zero `exitCode` instead of rejecting, so a
 * caller can report several independent installs without one bad repository
 * aborting the rest. A missing executable is reported the same way, which is
 * what happens when `npx` is unavailable.
 *
 * @param spec The command, its arguments, and a label for output.
 * @param options.streamOutput Pipe the command's output to this terminal
 *   instead of capturing it. Use it for slow commands whose progress the user
 *   should see; `stdout` is then empty.
 * @returns The exit code and whatever output was captured.
 */
export function runCommand(
  spec: Readonly<CommandSpec>,
  options: Readonly<{ streamOutput?: boolean }> = {},
): Promise<CommandResult> {
  const { streamOutput = false } = options;
  return new Promise((resolve) => {
    const child = spawn(spec.command, spec.args, {
      stdio: streamOutput
        ? ["ignore", "inherit", "inherit"]
        : ["ignore", "pipe", "pipe"],
    });

    const stdoutChunks: string[] = [];
    const stderrChunks: string[] = [];
    child.stdout?.setEncoding("utf8");
    child.stdout?.on("data", (chunk: string) => {
      stdoutChunks.push(chunk);
    });
    child.stderr?.setEncoding("utf8");
    child.stderr?.on("data", (chunk: string) => {
      stderrChunks.push(chunk);
    });

    child.on("error", (error: Error) => {
      resolve({
        exitCode: 1,
        stdout: stdoutChunks.join(""),
        stderr: `${stderrChunks.join("")}${error.message}`,
      });
    });

    child.on("close", (code) => {
      resolve({
        exitCode: code ?? 1,
        stdout: stdoutChunks.join(""),
        stderr: stderrChunks.join(""),
      });
    });
  });
}
