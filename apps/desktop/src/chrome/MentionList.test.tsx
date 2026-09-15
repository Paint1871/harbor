// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { MentionList, mentionKeydown, mentionQuery } from "./MentionList";

afterEach(cleanup);

const items = [
  { id: "readme", label: "README.md" },
  { id: "notes", label: "notes.txt" },
];

it("mentionQuery reads a trailing @ token", () => {
  expect(mentionQuery("see @src")).toBe("src");
  expect(mentionQuery("@")).toBe("");
  expect(mentionQuery("hello @fi")).toBe("fi");
  expect(mentionQuery("no mention")).toBe(null);
  expect(mentionQuery("keep going ")).toBe(null);
  expect(mentionQuery("user@host")).toBe(null);
});

it("arrow then enter calls onPick with the active id", () => {
  const onPick = vi.fn();
  const preventDefault = vi.fn();
  const moved = mentionKeydown({ key: "ArrowDown", preventDefault }, items, 0);
  expect(moved?.pick).toBeUndefined();
  expect(preventDefault).toHaveBeenCalled();

  render(<MentionList items={items} label="Files" onPick={onPick} />);
  const list = screen.getByRole("listbox", { name: "Files" });
  expect(list.getAttribute("aria-activedescendant")).toBeTruthy();
  expect(screen.getByRole("option", { name: "README.md" }).getAttribute("aria-selected")).toBe("true");

  fireEvent.keyDown(window, { key: "ArrowDown" });
  fireEvent.keyDown(window, { key: "Enter" });
  expect(onPick).toHaveBeenCalledTimes(1);
  expect(onPick).toHaveBeenCalledWith(items[moved!.index]!.id);
});

it("click still picks a mention", () => {
  const onPick = vi.fn();
  render(<MentionList items={items} label="Files" onPick={onPick} />);
  fireEvent.click(screen.getByRole("button", { name: "notes.txt" }));
  expect(onPick).toHaveBeenCalledWith("notes");
});

it("escape clears without picking", () => {
  const onPick = vi.fn();
  const onCancel = vi.fn();
  render(<MentionList items={items} label="Files" onPick={onPick} onCancel={onCancel} />);
  fireEvent.keyDown(window, { key: "Escape" });
  expect(onPick).not.toHaveBeenCalled();
  expect(onCancel).toHaveBeenCalledTimes(1);
});
