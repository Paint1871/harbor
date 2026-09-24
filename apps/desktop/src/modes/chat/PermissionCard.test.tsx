// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { PermissionCard, parsePermissionEvent, permissionShortcut, type PermissionRequest } from "./PermissionCard";

afterEach(cleanup);

const options = [
  { kind: "allow_always", optionId: "always" },
  { kind: "allow_once", optionId: "once" },
  { kind: "reject_once", optionId: "deny" },
];

const request: PermissionRequest = {
  id: "perm",
  title: "Read a file",
  path: "/tmp/project/README.md",
  options: [
    { optionId: "always", kind: "allow_always", name: "Always" },
    { optionId: "once", kind: "allow_once", name: "Allow" },
    { optionId: "deny", kind: "reject_once", name: "Deny" },
  ],
};

it("permissionShortcut prefers allow_once on Enter and a", () => {
  expect(permissionShortcut("Enter", options)).toEqual({ optionId: "once", cancelled: false });
  expect(permissionShortcut("a", options)).toEqual({ optionId: "once", cancelled: false });
  expect(permissionShortcut("A", options)).toEqual({ optionId: "once", cancelled: false });
});

it("permissionShortcut falls back to the first allow_* option", () => {
  expect(permissionShortcut("Enter", [{ kind: "allow_always", optionId: "session" }])).toEqual({
    optionId: "session",
    cancelled: false,
  });
});

it("permissionShortcut denies or stops on Escape, n, and d", () => {
  expect(permissionShortcut("Escape", options)).toEqual({ optionId: "deny", cancelled: false });
  expect(permissionShortcut("n", options)).toEqual({ optionId: "deny", cancelled: false });
  expect(permissionShortcut("d", options)).toEqual({ optionId: "deny", cancelled: false });
  expect(permissionShortcut("Escape", [{ kind: "allow_once", optionId: "once" }])).toEqual({
    optionId: null,
    cancelled: true,
  });
});

it("permissionShortcut ignores other keys", () => {
  expect(permissionShortcut("x", options)).toBeNull();
  expect(permissionShortcut(" ", options)).toBeNull();
  expect(permissionShortcut("Tab", options)).toBeNull();
});

it("parsePermissionEvent reads a request payload", () => {
  expect(parsePermissionEvent({
    id: "perm",
    title: "Run a command",
    command: "ls",
    options: [{ optionId: "once", kind: "allow_once", name: "Allow" }],
  })).toEqual({
    id: "perm",
    title: "Run a command",
    path: undefined,
    command: "ls",
    options: [{ optionId: "once", kind: "allow_once", name: "Allow" }],
    sessionRef: undefined,
  });
  expect(parsePermissionEvent(null)).toBeNull();
});

it("resolves Allow from Enter while the card is mounted", () => {
  const onResolve = vi.fn();
  render(<PermissionCard request={request} onResolve={onResolve} />);
  expect(screen.getByRole("button", { name: "Allow" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Allow for session" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Deny" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Stop" })).toBeTruthy();
  fireEvent.keyDown(window, { key: "Enter" });
  expect(onResolve).toHaveBeenCalledTimes(1);
  expect(onResolve).toHaveBeenCalledWith("once", false);
});

it("steals Enter from a composer textarea", () => {
  const onResolve = vi.fn();
  render(<PermissionCard request={request} onResolve={onResolve} />);
  const composer = document.createElement("textarea");
  composer.setAttribute("aria-label", "Message");
  document.body.appendChild(composer);
  fireEvent.keyDown(composer, { key: "Enter" });
  expect(onResolve).toHaveBeenCalledWith("once", false);
  composer.remove();
});

it("does not steal letter keys from a text field", () => {
  const onResolve = vi.fn();
  render(<PermissionCard request={request} onResolve={onResolve} />);
  const composer = document.createElement("textarea");
  document.body.appendChild(composer);
  fireEvent.keyDown(composer, { key: "a" });
  fireEvent.keyDown(composer, { key: "n" });
  fireEvent.keyDown(composer, { key: "d" });
  expect(onResolve).not.toHaveBeenCalled();
  composer.remove();
});

it("resolves only the newest card when several are mounted", () => {
  const first = vi.fn();
  const second = vi.fn();
  render(
    <>
      <PermissionCard request={{ ...request, id: "older", title: "Older" }} onResolve={first} />
      <PermissionCard request={{ ...request, id: "newer", title: "Newer" }} onResolve={second} />
    </>,
  );
  fireEvent.keyDown(window, { key: "a" });
  expect(first).not.toHaveBeenCalled();
  expect(second).toHaveBeenCalledTimes(1);
  expect(second).toHaveBeenCalledWith("once", false);
});

it("hands the shortcuts to the next card after the newest unmounts", () => {
  const first = vi.fn();
  const second = vi.fn();
  const view = render(
    <>
      <PermissionCard request={{ ...request, id: "older", title: "Older" }} onResolve={first} />
      <PermissionCard request={{ ...request, id: "newer", title: "Newer" }} onResolve={second} />
    </>,
  );
  view.rerender(
    <PermissionCard request={{ ...request, id: "older", title: "Older" }} onResolve={first} />,
  );
  fireEvent.keyDown(window, { key: "d" });
  expect(second).not.toHaveBeenCalled();
  expect(first).toHaveBeenCalledTimes(1);
  expect(first).toHaveBeenCalledWith("deny", false);
});
