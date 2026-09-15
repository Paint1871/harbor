// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { Transcript, transcriptCopyText, type TranscriptLine } from "./Transcript";

const userLine: TranscriptLine = { id: "u1", text: "Find launch leads", role: "user" };
const assistantLine: TranscriptLine = { id: "a1", text: "Here is a list.", role: "assistant" };
const mailLine: TranscriptLine = { id: "m1", text: "Release is tagged.", role: "mail" };

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("returns the on-screen string for user, assistant, and mail lines", () => {
  expect(transcriptCopyText(userLine)).toBe(userLine.text);
  expect(transcriptCopyText(assistantLine)).toBe(assistantLine.text);
  expect(transcriptCopyText(mailLine)).toContain(mailLine.text);
  render(<Transcript lines={[userLine, assistantLine, mailLine]} />);
  expect(screen.getByText(transcriptCopyText(userLine), { selector: ".harbor-bubble-user" })).toBeTruthy();
  expect(screen.getByText(transcriptCopyText(assistantLine), { selector: ".harbor-assistant-block" })).toBeTruthy();
  expect(screen.getByText(transcriptCopyText(mailLine), { selector: "[data-role='Handoff']" })).toBeTruthy();
});

it("copies each line through the clipboard API using transcriptCopyText", async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal("navigator", { clipboard: { writeText } });
  render(<Transcript lines={[userLine, assistantLine, mailLine]} />);
  const buttons = screen.getAllByRole("button", { name: "Copy message" });
  expect(buttons).toHaveLength(3);
  fireEvent.click(buttons[0]!);
  fireEvent.click(buttons[1]!);
  fireEvent.click(buttons[2]!);
  await waitFor(() => expect(writeText).toHaveBeenCalledTimes(3));
  expect(writeText).toHaveBeenNthCalledWith(1, transcriptCopyText(userLine));
  expect(writeText).toHaveBeenNthCalledWith(2, transcriptCopyText(assistantLine));
  expect(writeText).toHaveBeenNthCalledWith(3, transcriptCopyText(mailLine));
  expect(await screen.findAllByRole("button", { name: "Copied" })).toHaveLength(3);
});

it("falls back to execCommand when clipboard writeText is missing", async () => {
  vi.stubGlobal("navigator", { clipboard: undefined });
  let copied = "";
  document.execCommand = ((command: string) => {
    if (command === "copy") copied = document.querySelector("textarea")?.value ?? "";
    return true;
  }) as typeof document.execCommand;
  render(<Transcript lines={[mailLine]} />);
  fireEvent.click(screen.getByRole("button", { name: "Copy message" }));
  await waitFor(() => expect(copied).toBe(transcriptCopyText(mailLine)));
  expect(document.querySelector("textarea")).toBeNull();
});
