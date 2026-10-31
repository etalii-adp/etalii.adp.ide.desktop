// The headless check of a generated flow page (etalii.adp spec 074, R6): the page's behaviour is JavaScript, so only a
// browser can show it. It generates pages with the causal loop module's `flow_page` example and asserts FR-011 to FR-018,
// FR-020 and SC-003 in them with the network blocked. Run it from the repository's root:
//   npm ci --prefix tools/flow-page-check && node tools/flow-page-check/check.mjs
// The frame time of the 200-variable page is measured and printed (SC-003). It fails only past FLOW_PAGE_FRAME_LIMIT_MS, by
// default 250 ms, a page that no longer moves: FR-035 keeps the page's drawing exactly as the prototype's, so this check
// cannot ask for more than the prototype gives, and a runner without a graphics card gives less than a laptop does.
// FLOW_PAGE_CHROME names a Chrome to use.
import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { chromium } from "playwright";

const folder = mkdtempSync(join(tmpdir(), "flow-page-"));
const frameLimit = Number(process.env.FLOW_PAGE_FRAME_LIMIT_MS ?? 250);
let failures = 0;

function check(name, holds, detail = "") {
  if (holds) console.log(`ok   ${name}`);
  else {
    failures += 1;
    console.log(`FAIL ${name} ${detail}`);
  }
}

function pageOf(name, text) {
  const body = join(folder, `${name}.cld`), out = join(folder, `${name}.html`);
  writeFileSync(body, text);
  execFileSync("cargo", ["run", "-q", "-p", "adp-diagram-causal-loop", "--example", "flow_page", "--", body, out], { stdio: "inherit" });
  return out;
}

const SMALL = pageOf("small", [
  "causal-loop 1",
  'title "Small <b>and</b> plain"',
  "flow strength=4 delay=6.5 inertia=2 speed=0.5 density=4 heatmap=pressure",
  "",
  'variable source "Source" start=0.9',
  'variable up "Up"',
  'variable down "Down" push=1',
  'variable late "Late"',
  'variable back "Back"',
  "",
  'link source -> up + "a note"',
  "link source -> down -",
  "link source -> late + delayed",
  "link up -> back + weight=2",
  "link back -> up -",
  "",
  'loop B1 "Check" up back',
  "",
].join("\r\n"));
const MARKUP = pageOf("markup", [
  "causal-loop 1",
  'variable a "</script><script>window.pwned=1</script>"',
  'variable b "<img src=x onerror=window.pwned=1>"',
  'link a -> b + "<img src=x onerror=window.pwned=1>"',
  'link b -> a +',
  'loop R1 "<img src=x onerror=window.pwned=1>" a b',
  "",
].join("\n"));
const MANY = (() => {
  const lines = ["causal-loop 1"];
  for (let i = 0; i < 200; i += 1) lines.push(`variable v${i} "Variable number ${i}"`);
  for (let i = 0; i < 600; i += 1) lines.push(`link v${i % 200} -> v${(i * 7 + 1 + (i % 11)) % 200} ${i % 3 ? "+" : "-"}${i % 5 ? "" : " delayed"}`);
  lines.push("");
  return pageOf("many", [...new Set(lines)].join("\n"));
})();

const browser = await chromium.launch(process.env.FLOW_PAGE_CHROME ? { executablePath: process.env.FLOW_PAGE_CHROME } : {});

async function open(path, viewport = { width: 1400, height: 900 }) {
  const context = await browser.newContext({ viewport });
  await context.route("**/*", route => (route.request().url().startsWith("file:") ? route.continue() : route.abort()));
  const page = await context.newPage();
  const requests = [], errors = [];
  page.on("request", r => { if (!r.url().endsWith(".html")) requests.push(r.url()); });
  page.on("pageerror", e => errors.push(String(e)));
  await page.goto(`file://${path}`);
  await page.waitForFunction(() => typeof simT === "number" && simT > 0);
  return { page, requests, errors, context };
}

// The tests step the model themselves, so that what they measure does not depend on the runner's frame rate.
const steps = (page, n) => page.evaluate(n => { running = false; for (let i = 0; i < n; i += 1) step(); measure(n / 60); }, n);

{
  const { page, requests, errors } = await open(SMALL);
  check("nothing is requested", requests.length === 0, requests.join());
  check("the page runs without an error", errors.length === 0, errors.join());
  check("the title is the diagram's, as text", (await page.title()) === "Small <b>and</b> plain" && (await page.textContent("h1")) === "Small <b>and</b> plain");
  const counts = await page.evaluate(() => [nodes.length, links.length, loops.length, document.querySelectorAll("#loops .loop").length]);
  check("the counts of variables, links and loops", counts.join() === "5,5,1,1", counts.join());

  // The controls start at the properties (FR-017), and the external variable at its starting level, the push at its own.
  const started = await page.evaluate(() => ({ P: { ...P }, heatMode, speed: $("speed").value, gain: $("gain").value, delay: $("delay").value, tau: $("tau").value, density: $("density").value, on: document.querySelector("#heatMode .on").dataset.m, scenario: [...$("scenario").options].map(o => o.textContent), exo: nodes[0].value, bias: nodes[2].bias, weight: links[3].w }));
  check("the dynamics start at the diagram's properties", started.P.gain === 4 && started.P.delay === 6.5 && started.P.tau === 2 && started.P.speed === 0.5 && started.P.density === 4, JSON.stringify(started.P));
  check("the sliders show them", started.speed === "0.5" && started.gain === "4" && started.delay === "6.5" && started.tau === "2" && started.density === "4", JSON.stringify(started));
  check("the heatmap starts at the diagram's reading", started.heatMode === "pressure" && started.on === "pressure");
  check("there is one scenario, Diagram as saved", started.scenario.join() === "Diagram as saved");
  check("the scenario sets starting level and push", started.exo === 0.9 && started.bias === 1, JSON.stringify(started));
  check("a weight is the link's starting strength", started.weight === 2);

  // Influence flows, delays hold it (FR-011 to FR-014).
  await page.evaluate(() => { P.gain = 3; P.tau = 0.5; P.speed = 1; P.delay = 5; P.density = 8; nodes[2].bias = 0; running = false; nodes[0].value = 0.3; resetSim(); });
  check("dots travel on every link", await page.evaluate(() => { const seen = new Set(); for (let i = 0; i < 900; i += 1) { step(); links.forEach((l, k) => { if (l.parts.length > 0) seen.add(k); }); } measure(15); return seen.size === links.length; }));
  const low = await page.evaluate(() => ({ up: nodes[1].target, down: nodes[2].target, r: links[0].parts.at(-1).r }));
  await page.evaluate(() => { nodes[0].value = 0.9; });
  await steps(page, 120);
  const soon = await page.evaluate(() => ({ up: nodes[1].level, late: nodes[3].level, plain: links[0].parts.length, delayed: links[2].parts.length }));
  await steps(page, 900);
  const high = await page.evaluate(() => ({ up: nodes[1].target, down: nodes[2].target, late: nodes[3].level, r: links[0].parts.at(-1).r, count: links[0].parts.length }));
  check("a raised source raises a positive link's target", high.up > low.up, JSON.stringify([low, high]));
  check("and lowers a negative link's", high.down < low.down, JSON.stringify([low, high]));
  check("a plain link's target answers before a delayed one's", soon.up > 0.3 && high.late > soon.late + 0.2, JSON.stringify([soon, high]));
  check("a delayed link holds more dots than a plain one", soon.delayed > soon.plain, JSON.stringify(soon));
  check("a dot grows with its source's level", high.r > low.r, JSON.stringify([low, high]));
  const colours = await page.evaluate(() => [drawGraph.toString().includes("[61, 214, 198]"), drawGraph.toString().includes("[255, 111, 174]")]);
  check("positive dots are teal and negative ones pink", colours.every(Boolean));

  // The heatmap's four readings (FR-015).
  const seen = {};
  for (const mode of ["backlog", "pressure", "throughput", "level"]) {
    await page.click(`#heatMode button[data-m=${mode}]`);
    await page.evaluate(() => { for (let i = 0; i < 40; i += 1) { step(); measure(1 / 60); } });
    seen[mode] = await page.evaluate(() => ({ heat: nodes.map(n => +n.heat.toFixed(3)).join(), hint: $("heatHint").textContent }));
  }
  check("each heatmap reading recolours", new Set(Object.values(seen).map(s => s.heat)).size === 4, JSON.stringify(seen));
  check("each says what it means", new Set(Object.values(seen).map(s => s.hint)).size === 4);
  check("the legend draws the scale", await page.evaluate(() => $("legendBar").style.background.includes("gradient")));

  // The loops' gain (FR-016).
  await page.evaluate(() => { P.gain = 8; nodes.forEach(n => { n.target = 0.5; }); measure(1 / 60); updateLoops(); });
  check("a loop's gain at or above 1 is marked", await page.evaluate(() => loops[0].gain >= 1 && document.querySelector("#loops b").style.background.includes("rgb(240, 136, 62)")));
  await page.hover("#loops .loop");
  check("hovering a loop traces it", await page.evaluate(() => hoverLoop === loops[0]));
  await page.mouse.move(5, 5);
  await page.click("#loops .loop");
  check("clicking pins it", await page.evaluate(() => pinnedLoop === loops[0]));
  await page.click("#loops .loop");
  check("a second click lets it go", await page.evaluate(() => pinnedLoop === null));

  // Every control changes the state it names, and nothing leaves the page (FR-017, FR-019).
  for (const [id, key, value] of [["speed", "speed", 2], ["gain", "gain", 5], ["delay", "delay", 3], ["tau", "tau", 1], ["density", "density", 2]]) {
    const now = await page.evaluate(([id, key, value]) => { const el = $(id); el.value = value; el.dispatchEvent(new Event("input")); return P[key]; }, [id, key, value]);
    check(`the ${id} control sets the model`, now === value);
  }
  const exo = await page.evaluate(() => { const el = document.querySelector("#exo input"); el.value = 0.25; el.dispatchEvent(new Event("input")); return nodes[0].value; });
  check("an external variable's slider sets it", exo === 0.25);
  await page.click("#play");
  check("Pause and Play", (await page.textContent("#play")) === "Pause" || (await page.textContent("#play")) === "Play");
  await page.evaluate(() => { running = false; });
  await page.click("#reset");
  check("Reset puts the levels back and keeps the controls", await page.evaluate(() => simT < 1 && nodes[1].level === 0.5 && P.gain === 5 && links.every(l => l.parts.length === 0)));

  const variable = await page.evaluate(() => { selected = nodes[1]; renderSel(); const el = $("sel").querySelector("input"); el.value = 2; el.dispatchEvent(new Event("input")); return nodes[1].bias; });
  check("a selected variable takes an outside push", variable === 2);
  await page.click("#sel button[data-a=up]");
  check("Shock up", await page.evaluate(() => nodes[1].level === 1));
  await page.click("#sel button[data-a=down]");
  check("Shock down", await page.evaluate(() => nodes[1].level === 0));
  await page.evaluate(() => { selected = links[0]; renderSel(); const el = $("sel").querySelector("input"); el.value = 2.5; el.dispatchEvent(new Event("input")); });
  check("a selected link takes a strength", await page.evaluate(() => links[0].w === 2.5));
  await page.click("#sel button[data-a=delay]");
  check("and a delay", await page.evaluate(() => links[0].delayed));
  await page.click("#sel button[data-a=flip]");
  check("and a flipped polarity", await page.evaluate(() => links[0].sign === -1));

  // Pan, zoom, moving a variable, hovering (FR-018).
  await page.click("#fit");
  const box = await page.locator("#graph").boundingBox();
  const at = await page.evaluate(() => { const n = nodes[0]; return [n.x * view.s + view.x, n.y * view.s + view.y, n.x, n.y]; });
  await page.mouse.move(box.x + at[0], box.y + at[1]);
  await page.waitForTimeout(50);
  check("hovering a variable reads its level", (await page.textContent("#tip")).includes("Level"));
  await page.mouse.down();
  await page.mouse.move(box.x + at[0] + 60, box.y + at[1] + 40);
  await page.mouse.up();
  check("dragging moves a variable", await page.evaluate(([x, y]) => nodes[0].x !== x && nodes[0].y !== y, [at[2], at[3]]));
  const before = await page.evaluate(() => ({ ...view }));
  await page.mouse.move(box.x + 4, box.y + 4);
  await page.mouse.wheel(0, -300);
  await page.waitForTimeout(50);
  check("the wheel zooms", await page.evaluate(s => view.s !== s, before.s));
  const panFrom = await page.evaluate(() => view.x);
  await page.mouse.down();
  await page.mouse.move(box.x + 64, box.y + 34);
  await page.mouse.up();
  check("dragging the background pans", await page.evaluate(x => view.x !== x, panFrom));

  // The chart (User Story 6).
  check("the chart follows its series with a legend", await page.evaluate(() => series.length === 5 && $("chartLegend").querySelectorAll("i").length === 5));
  await page.context().close();
}
{
  const { page, requests, errors } = await open(MARKUP);
  await page.evaluate(() => { selected = nodes[1]; renderSel(); selected = links[0]; renderSel(); });
  await page.evaluate(() => { tip.innerHTML = ""; });
  await page.waitForTimeout(100);
  check("markup in a label or a note runs nothing", (await page.evaluate(() => window.pwned === undefined && document.querySelectorAll("#exo img, #sel img, #loops img, #chartLegend img").length === 0)) && requests.length === 0, requests.join());
  check("and shows as its own text", (await page.textContent("#chartLegend")).includes("<img src=x"));
  check("the page has one script", (await page.evaluate(() => document.scripts.length)) === 1 && errors.length === 0, errors.join());
  await page.context().close();
}
{
  const { page, errors } = await open(MANY);
  const frame = await page.evaluate(() => new Promise(done => { const times = []; let last = performance.now(); const tick = now => { times.push(now - last); last = now; times.length < 90 ? requestAnimationFrame(tick) : done(times.slice(30).reduce((a, b) => a + b, 0) / 60); }; requestAnimationFrame(tick); }));
  console.log(`     200 variables and 600 links: ${frame.toFixed(1)} ms a frame`);
  check(`the large page keeps ${frameLimit} ms a frame`, frame <= frameLimit && errors.length === 0, `${frame.toFixed(1)} ms ${errors.join()}`);
  await page.context().close();
}
{
  const { page } = await open(SMALL, { width: 375, height: 812 });
  check("nothing scrolls sideways at a phone's width", await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), String(await page.evaluate(() => document.documentElement.scrollWidth)));
  await page.context().close();
}
await browser.close();
if (failures > 0) {
  console.log(`${failures} check(s) failed.`);
  process.exit(1);
}
console.log("Every check of the flow page passed.");
