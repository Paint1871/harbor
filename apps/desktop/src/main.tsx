import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "@harbor/ui/ThemeProvider";
import { forceDark } from "@harbor/ui/theme";
import { App } from "./App";
import "@harbor/ui/tokens.css";
import "@harbor/ui/primitives.css";
import "./app.css";

forceDark();
const root = document.getElementById("root");
if (!root) throw new Error("Harbor root is missing");
const overlay = new URLSearchParams(window.location.search).get("window") === "overlay";
if (overlay) document.documentElement.dataset.window = "overlay";
createRoot(root).render(
  <StrictMode>
    <ThemeProvider>{overlay ? null : <App />}</ThemeProvider>
  </StrictMode>,
);
