// Open a URL in headless Brave over CDP and report what a visitor gets:
// the title, whether the crawler summary left, which selectors rendered.
//
//   node scripts/check-page.mjs http://127.0.0.1:8099/plot/sinwood-7/ .plot-hero .deal-now
//
// Needs Node ≥ 22 (global WebSocket) and Brave; starts its own Brave on a
// spare port with a throwaway profile so the user's browser is untouched.
import { spawn } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const BRAVE = "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser";
const PORT = 9227;
const [url, ...selectors] = process.argv.slice(2);
if (!url) {
  console.error("usage: node scripts/check-page.mjs <url> [selector ...]");
  process.exit(2);
}

const profile = mkdtempSync(join(tmpdir(), "cyberia-cdp-"));
const brave = spawn(
  BRAVE,
  [`--headless=new`, `--remote-debugging-port=${PORT}`, `--user-data-dir=${profile}`, "--no-first-run", "--window-size=1280,900", "about:blank"],
  { stdio: "ignore" },
);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function cdp() {
  for (let i = 0; i < 40; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/json/new?${url}`, { method: "PUT" });
      return await r.json();
    } catch {
      await sleep(250);
    }
  }
  throw new Error("brave did not come up");
}

try {
  const page = await cdp();
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((r) => (ws.onopen = r));
  let id = 0;
  const pending = new Map();
  ws.onmessage = (ev) => {
    const m = JSON.parse(ev.data);
    if (m.id && pending.has(m.id)) {
      pending.get(m.id)(m.result);
      pending.delete(m.id);
    }
  };
  const send = (method, params = {}) =>
    new Promise((r) => {
      pending.set(++id, r);
      ws.send(JSON.stringify({ id, method, params }));
    });
  const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, returnByValue: true })).result.value;

  await send("Page.enable");
  await send("Runtime.enable");
  await sleep(4000);
  const report = await evaluate(`(() => ({
    url: location.href,
    title: document.title,
    seo_left: !document.getElementById("plot-seo"),
    body_children: document.body.children.length,
    found: ${JSON.stringify(selectors)}.map(s => [s, document.querySelectorAll(s).length]),
    hero: (document.querySelector(".cities-title")||{}).textContent || null,
    lead: (document.querySelector(".cities-lead")||{}).textContent || null,
  }))()`);
  console.log(JSON.stringify(report, null, 1));
  const shot = process.env.SHOT;
  if (shot) {
    const { data } = await send("Page.captureScreenshot", { format: "png" });
    const { writeFileSync } = await import("node:fs");
    writeFileSync(shot, Buffer.from(data, "base64"));
    console.log("screenshot →", shot);
  }
  ws.close();
} finally {
  brave.kill();
}
