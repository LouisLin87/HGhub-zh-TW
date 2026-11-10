import React from "react";
import { createRoot } from "react-dom/client";
import { AppProvider } from "../src/store";
import Search from "../src/pages/Search";
import Profile from "../src/pages/Profile";
import Settings from "../src/pages/Settings";
import "../src/index.css";

const fixture = { calls: [] as { cmd: string; args: Record<string, unknown> }[] };
Object.assign(window, {
  localeFixture: fixture,
  __TAURI_INTERNALS__: { invoke(cmd: string, args: Record<string, unknown> = {}) {
    fixture.calls.push({ cmd, args });
    if (cmd === "app_boot") return Promise.resolve({ user: null, logged_in: false, strict_security: true });
    if (cmd === "line_state") return Promise.resolve({ base: "https://fixture.invalid" });
    if (cmd === "playback_log_status") return Promise.resolve({ enabled: false, path: null, error: null });
    if (cmd === "debug_log") return Promise.resolve();
    if (cmd === "api_search") return Promise.resolve({ items: [], keyword: args.q, pagination: { page: 1, pages: 1, size: 20, total: 0 } });
    return Promise.reject(`本機驗證不允許此 IPC：${cmd}`);
  } },
});
const mode = new URLSearchParams(location.search).get("page");
const wide = matchMedia("(min-width: 900px)").matches;
createRoot(document.querySelector("#root")!).render(
  <AppProvider><div style={{ position: "relative", width: "100%", height: "100vh" }}>
    {mode === "settings" ? <Settings onBack={() => {}} wide={wide} />
      : mode === "profile" ? <Profile wide={wide} /> : <Search active wide={wide} />}
  </div></AppProvider>,
);
