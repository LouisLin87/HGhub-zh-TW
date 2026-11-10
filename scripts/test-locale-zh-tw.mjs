import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { chromium } from "playwright-core";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "test-results", "locale-zh-tw", new Date().toISOString().replace(/[:.]/g, "-"));
await mkdir(output, { recursive: true });
const report = { passed: [], external: [], errors: [] };
const vite = await createServer({ root, server: { host: "127.0.0.1", port: 0, strictPort: false, hmr: false } });
let browser;
try {
  await vite.listen();
  const origin = vite.resolvedUrls.local[0];
  browser = await chromium.launch({ headless: true, executablePath: process.env.BROWSER_EXECUTABLE || undefined });
  const page = await browser.newPage();
  page.on("pageerror", error => report.errors.push(error.message));
  await page.route("**/*", route => {
    const url = new URL(route.request().url());
    if (url.origin === new URL(origin).origin) return route.continue();
    report.external.push(url.origin);return route.abort();
  });
  await page.addInitScript(() => localStorage.clear());
  for (const width of [320, 768, 1024, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    for (const mode of ["search", "settings", "profile"]) {
      await page.goto(`${origin}tests/locale-zh-tw.html?page=${mode}`);
      const expected = mode === "search" ? "熱門題材" : mode === "settings" ? "網路診斷" : "未登入";
      await page.getByText(expected, { exact: true }).first().waitFor();
      assert.equal(await page.locator("html").getAttribute("lang"), "zh-Hant-TW");
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false, `${mode}/${width}: 不應水平溢位`);
      if (width === 320 || width === 1440) await page.screenshot({ path: join(output, `${mode}-${width}.png`), fullPage: true, animations: "disabled" });
    }
  }
  report.passed.push("搜尋、設定與個人頁在 320／768／1024／1440 寬度使用繁體文字，沒有水平溢位");
  await page.goto(`${origin}tests/locale-zh-tw.html?page=search`);
  await page.getByRole("button", { name: "逆襲", exact: true }).focus();
  await page.keyboard.press("Enter");
  await page.waitForFunction(() => window.localeFixture.calls.some(call => call.cmd === "api_search"));
  const call = await page.evaluate(() => window.localeFixture.calls.find(call => call.cmd === "api_search"));
  assert.equal(call.args.q, "逆袭", "繁體標籤仍應使用既有服務查詢值");
  assert.equal(call.args.page, 1);
  assert.equal(await page.getByPlaceholder("搜尋短劇、作者、題材…").inputValue(), "逆襲");
  await page.getByText("沒有找到相關內容", { exact: true }).waitFor();
  assert((await page.locator("body").innerText()).includes("「逆襲」共"));
  report.passed.push("熱門題材可用鍵盤操作，顯示繁體並維持原本的查詢值與空結果處理");
  assert.deepEqual(report.external, [], "驗證不得存取上游");
  assert.deepEqual(report.errors, [], "介面不得出現執行錯誤");
  console.log(JSON.stringify(report, null, 2));
} catch(error) {
  report.failure = String(error.stack || error);
  console.error(report.failure);
  process.exitCode = 1;
} finally {
  await writeFile(join(output, "report.json"), JSON.stringify(report, null, 2));
  await browser?.close();
  await vite.close();
  console.log(`驗證產物：${output}`);
}
