// The memory probe of the standalone host (etalii.adp spec 030, SC-007, research R12, task T091).
//
// Runs the standalone host as published (its backend service, serving the client's production build) with Google
// Chrome as its browser, opens the inputs `cargo bench -p adp-shell --bench memory` wrote, and reads the memory of the
// backend's process and of every Chrome process, the way Activity Monitor reads it (`footprint`), at the points the
// desktop host is read at: the project list, the workspace of 5,000 files open, and the workspace open with three
// documents in tabs (a Markdown file, a JSON file and the 1,000-element canvas, active), which is SC-007's point.
//
//   node parity/memory/standalone.mjs --publish <the standalone host's publish folder> --workspace <inputs>/workspace
//
// Optional: --settle <seconds each point is left idle before it is read, default 15>, --chrome <Chrome's program>,
// --port <the backend's port, default 5099>. macOS only, for `footprint`, with a screen, as Chrome opens a real
// window. The backend runs with a temporary HOME, so the person's own ADP project list is neither read nor changed,
// and with a user name and credential the probe makes up for this run; Chrome runs with a temporary profile, one
// window and one tab, which is the least a person's Chrome holds.
//
// Node 22 or later (for its WebSocket); no packages.

import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

const READS = 5;

function option(name, fallback) {
  const index = process.argv.indexOf(`--${name}`);
  return index >= 0 ? process.argv[index + 1] : fallback;
}

const publish = option("publish");
const workspace = option("workspace");
if (!publish || !workspace) {
  console.error("usage: node parity/memory/standalone.mjs --publish <folder> --workspace <folder>");
  process.exit(2);
}
const settle = Number(option("settle", "15")) * 1000;
const chromeProgram = option("chrome", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
const port = Number(option("port", "5099"));
const debugPort = port + 1;
const address = `http://127.0.0.1:${port}`;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function until(what, check, timeout = 120_000) {
  const started = Date.now();
  for (;;) {
    const value = await check().catch(() => undefined);
    if (value) {
      return value;
    }
    if (Date.now() - started > timeout) {
      throw new Error(`gave up waiting for ${what}`);
    }
    await sleep(250);
  }
}

// Every process descending from `pid`, itself included.
function family(pid) {
  const rows = execFileSync("ps", ["-A", "-o", "pid=,ppid="], { encoding: "utf8" })
    .trim()
    .split("\n")
    .map((line) => line.trim().split(/\s+/).map(Number));
  const found = new Set([pid]);
  for (let grew = true; grew; ) {
    grew = false;
    for (const [child, parent] of rows) {
      if (found.has(parent) && !found.has(child)) {
        found.add(child);
        grew = true;
      }
    }
  }
  return [...found];
}

// The physical footprint of each process in bytes, as Activity Monitor shows it as Memory.
function footprints(pids) {
  const text = execFileSync("footprint", ["-f", "bytes", "--noCategories", ...pids.flatMap((pid) => ["-p", String(pid)])], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
  return [...text.matchAll(/phys_footprint: (\d+) B/g)].reduce((sum, match) => sum + Number(match[1]), 0);
}

function residents(pids) {
  const text = execFileSync("ps", ["-o", "rss=", "-p", pids.join(",")], { encoding: "utf8" });
  return text
    .trim()
    .split("\n")
    .reduce((sum, line) => sum + Number(line.trim()) * 1024, 0);
}

const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];
const mib = (bytes) => `${(bytes / 1024 / 1024).toFixed(1)} MiB`;

async function read(what, backend, chrome) {
  await sleep(settle);
  const reads = { backend: [], chrome: [], backendResident: [], chromeResident: [], processes: [] };
  for (let i = 0; i < READS; i++) {
    if (i > 0) {
      await sleep(1000);
    }
    const browser = family(chrome.pid);
    reads.backend.push(footprints([backend.pid]));
    reads.chrome.push(footprints(browser));
    reads.backendResident.push(residents([backend.pid]));
    reads.chromeResident.push(residents(browser));
    reads.processes.push(browser.length);
  }
  const b = median(reads.backend);
  const c = median(reads.chrome);
  const br = median(reads.backendResident);
  const cr = median(reads.chromeResident);
  console.log(
    `  ${what}: footprint ${mib(b + c)} (backend ${mib(b)}, Chrome ${mib(c)} in ${median(reads.processes)} processes), ` +
      `resident ${mib(br + cr)} (backend ${mib(br)}, Chrome ${mib(cr)}) (median of ${READS} reads)`,
  );
}

// A connection to the page through Chrome's DevTools protocol.
async function devtools() {
  const targets = await until("Chrome's page", async () => {
    const list = await (await fetch(`http://127.0.0.1:${debugPort}/json/list`)).json();
    return list.find((t) => t.type === "page" && t.url.startsWith(address));
  });
  const socket = new WebSocket(targets.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.onopen = resolve;
    socket.onerror = reject;
  });
  let next = 0;
  const waiting = new Map();
  socket.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (waiting.has(message.id)) {
      waiting.get(message.id)(message);
      waiting.delete(message.id);
    }
  };
  const send = (method, params = {}) =>
    new Promise((resolve) => {
      const id = ++next;
      waiting.set(id, resolve);
      socket.send(JSON.stringify({ id, method, params }));
    });
  const evaluate = async (expression) => {
    const answer = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (answer.result?.exceptionDetails) {
      throw new Error(answer.result.exceptionDetails.text);
    }
    return answer.result?.result?.value;
  };
  return { evaluate, close: () => socket.close() };
}

// What the page script is given: setting a React input's value, finding an explorer entry by its path, and clicking.
const helpers = `
  window.__probe = {
    type(input, value) {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
      setter.call(input, value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    },
    entry(path) {
      let scope = document.querySelector("[role=tree]");
      let item = null;
      for (const name of path) {
        if (!scope) return null;
        item = [...scope.querySelectorAll("li[role=treeitem]")].find(
          (li) => li.querySelector(":scope > .explorer-tree-row .explorer-tree-node-name")?.textContent === name);
        if (!item) return null;
        scope = item;
      }
      return item;
    },
    expand(path) {
      const item = this.entry(path);
      if (!item) return false;
      if (item.getAttribute("aria-expanded") === "true") return !!item.querySelector(":scope > ul");
      item.querySelector(":scope > .explorer-tree-row .explorer-tree-chevron-button")?.click();
      return false;
    },
    open(path) {
      const item = this.entry(path);
      if (!item) return false;
      item.querySelector(":scope > .explorer-tree-row").dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
      return true;
    },
    tabs() {
      return [...document.querySelectorAll("[role=tab]")].map((tab) => tab.textContent);
    },
  };
  true`;

const temporary = mkdtempSync(join(tmpdir(), "adp-memory-"));
const home = join(temporary, "home");
mkdirSync(home);
const username = `probe-${randomBytes(4).toString("hex")}`;
const credential = randomBytes(18).toString("base64url");
const started = [];

function stop() {
  for (const child of started.reverse()) {
    try {
      child.kill("SIGTERM");
    } catch {
      // Already ended.
    }
  }
}

try {
  const backend = spawn("dotnet", ["EtAlii.Adp.Backend.Service.dll"], {
    cwd: publish,
    env: {
      ...process.env,
      HOME: home,
      XDG_CONFIG_HOME: join(home, ".config"),
      ASPNETCORE_ENVIRONMENT: "Production",
      ASPNETCORE_URLS: address,
      LocalAuthenticator__Username: username,
      LocalAuthenticator__Credential: credential,
    },
    stdio: "ignore",
  });
  started.push(backend);
  await until("the backend", async () => (await fetch(address)).ok);

  const chrome = spawn(
    chromeProgram,
    [
      `--user-data-dir=${join(temporary, "chrome")}`,
      `--remote-debugging-port=${debugPort}`,
      "--no-first-run",
      "--no-default-browser-check",
      "--disable-default-apps",
      "--window-size=1280,800",
      address,
    ],
    { stdio: "ignore" },
  );
  started.push(chrome);
  const page = await devtools();
  console.log(`SC-007: the standalone host in ${publish} and Chrome, idle, their memory read by footprint`);
  console.log(`  inputs in ${workspace}`);

  // Signing in.
  await until("the sign-in page", () => page.evaluate(`!!document.querySelector("form.auth-card")`));
  await page.evaluate(helpers);
  await page.evaluate(`(() => {
    const inputs = document.querySelectorAll("form.auth-card input");
    __probe.type(inputs[0], ${JSON.stringify(username)});
    __probe.type(inputs[1], ${JSON.stringify(credential)});
    return true; })()`);
  await page.evaluate(`document.querySelector("form.auth-card button[type=submit]").click(), true`);
  await until("the project list", () => page.evaluate(`!!document.querySelector("[aria-label='Add project']")`));
  await read("the project list", backend, chrome);

  // Adding the workspace as a project, and opening it.
  await page.evaluate(`document.querySelector("[aria-label='Add project']").click(), true`);
  await until("the add dialog", () => page.evaluate(`!!document.querySelector(".add-project-fields input")`));
  await page.evaluate(`(() => {
    const inputs = document.querySelectorAll(".add-project-fields input");
    __probe.type(inputs[0], "workspace");
    __probe.type(inputs[1], ${JSON.stringify(workspace)});
    return true; })()`);
  await page.evaluate(`[...document.querySelectorAll("button")].filter(b => b.textContent.trim() === "Add project").pop().click(), true`);
  await until("the project's card", () => page.evaluate(`!!document.querySelector(".project-card-open")`));
  await page.evaluate(`document.querySelector(".project-card-open").click(), true`);
  await until("the workspace's tree", () => page.evaluate(`!!(window.__probe && __probe.entry(["generated.dgr"]))`));
  await read("the workspace of 5,000 files open", backend, chrome);

  // The three documents: a Markdown file, a JSON file and the canvas, which is so the active tab, each found in the
  // explorer by expanding its folders, as a person opens them.
  for (const path of [
    ["area-00", "module-00", "item-0000.md"],
    ["area-01", "module-00", "item-0001.json"],
    ["generated.dgr"],
  ]) {
    for (let depth = 1; depth < path.length; depth++) {
      const folders = JSON.stringify(path.slice(0, depth));
      await until(`${path.slice(0, depth).join("/")} expanded`, () => page.evaluate(`__probe.expand(${folders})`));
    }
    await until(`${path.join("/")} in the tree`, () => page.evaluate(`!!__probe.entry(${JSON.stringify(path)})`));
    await page.evaluate(`__probe.open(${JSON.stringify(path)})`);
    const name = path[path.length - 1].replace(/\.dgr$/, "");
    await until(`${name}'s tab`, async () => (await page.evaluate(`__probe.tabs()`)).some((t) => t.includes(name)));
  }
  const tabs = await page.evaluate(`__probe.tabs()`);
  const shown = await until("the canvas drawn", () => page.evaluate(`document.body.textContent.includes("Element 0999")`), 60_000).catch(() => false);
  console.log(`  tabs: ${tabs.join(", ")}; the canvas's last element ${shown ? "is in the page" : "was not found in the page's text"}`);
  await read("the workspace open with three documents in tabs (SC-007)", backend, chrome);
  page.close();
} finally {
  stop();
  await sleep(2000);
  rmSync(temporary, { recursive: true, force: true });
}
