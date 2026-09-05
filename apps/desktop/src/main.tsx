import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { forceDark } from "@harbor/ui/theme";
import { App } from "./App";
import { FloatingPill } from "./overlay/FloatingPill";
import { NotchPill } from "./overlay/NotchPill";
import "@harbor/ui/tokens.css";
import "@harbor/ui/primitives.css";
import "./app.css";

forceDark();
const root = document.getElementById("root");
if (!root) throw new Error("Harbor root is missing");
const overlay = new URLSearchParams(window.location.search).get("window") === "overlay";
if (overlay) document.documentElement.dataset.window = "overlay";
const overlayView = overlay ? (
  matchMedia("(min-width: 300px)").matches ? (
    <NotchPill copy="Listening · release fn to send" />
  ) : (
    <FloatingPill copy="Listening · release fn to send" />
  )
) : null;
createRoot(root).render(<StrictMode>{overlay ? overlayView : <App />}</StrictMode>);
