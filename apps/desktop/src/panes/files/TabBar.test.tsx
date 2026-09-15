// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { TabBar } from "./TabBar";

afterEach(cleanup);

it("shows the basename, full path tooltip, and a dirty mark", () => {
  const onSelect = vi.fn();
  const onClose = vi.fn();
  render(
    <TabBar
      files={["/Users/ada/src/main.rs", "C:\\project\\app.py"]}
      active="/Users/ada/src/main.rs"
      dirty={["C:\\project\\app.py"]}
      onSelect={onSelect}
      onClose={onClose}
    />,
  );

  const rust = screen.getByRole("tab", { name: "main.rs" });
  expect(rust.textContent).toContain("main.rs");
  expect(rust.getAttribute("title")).toBe("/Users/ada/src/main.rs");
  expect(rust.getAttribute("data-dirty")).toBe("false");
  expect(rust.querySelector(".harbor-tab-dirty")).toBeNull();

  const python = screen.getByRole("tab", { name: "app.py, unsaved changes" });
  expect(python.querySelector(".harbor-tab-dirty")?.textContent).toBe("•");
  expect(python.querySelector(".harbor-tab-name")?.textContent).toBe("app.py");
  expect(python.getAttribute("title")).toBe("C:\\project\\app.py");
  expect(python.getAttribute("data-dirty")).toBe("true");

  fireEvent.click(python.querySelector(".harbor-tab-close")!);
  expect(onClose).toHaveBeenCalledWith("C:\\project\\app.py");
  expect(onSelect).not.toHaveBeenCalled();
});
