// `atd/validate` through the WebAssembly language server (etalii.adp spec 035, T162, FR-063, SC-004): every vendored
// description with conformance cases, its cases handed in as content and its files read through the host's
// functions, gives the result `atd validate --json` gives. Run after `wasm-bindgen --target nodejs`:
//
//     ATD=target/release/atd node crates/adp-atd-lsp/tests/wasm/validate.mjs <wasm-bindgen out dir>

import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { createRequire } from 'node:module';
import { basename, dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const glue = process.argv[2];
const atd = process.env.ATD;
if (!glue || !atd) {
  console.error('usage: ATD=<atd binary> node validate.mjs <wasm-bindgen out dir>');
  process.exit(2);
}
const root = resolve(dirname(new URL(import.meta.url).pathname), '../../../..');
const definitions = join(root, 'vendor/etalii.adp/definitions');
const { WasmServer } = createRequire(import.meta.url)(resolve(glue, 'adp_atd_lsp.js'));

const read = (path) => (existsSync(path) && statSync(path).isFile() ? readFileSync(path, 'utf8') : null);
const list = (folder) =>
  existsSync(folder) ? readdirSync(folder).filter((f) => f.endsWith('.atd')).sort().map((f) => join(folder, f)) : [];
const folders = (folder) =>
  existsSync(folder)
    ? readdirSync(folder).sort().map((f) => join(folder, f)).filter((f) => statSync(f).isDirectory())
    : [];

// The cases of each description: the folders `conformance/<type>/<case>/` beside it whose fixture names it.
const described = new Map();
for (const kind of folders(definitions)) {
  for (const type of folders(join(kind, 'conformance'))) {
    for (const folder of folders(type)) {
      const fixture = read(join(folder, 'conformance.json'));
      if (fixture === null) continue;
      const description = resolve(folder, JSON.parse(fixture).description);
      const cases = described.get(description) ?? [];
      const files = {};
      for (const file of readdirSync(folder).sort()) {
        files[file] = readFileSync(join(folder, file)).toString('base64');
      }
      cases.push({ name: basename(folder), files });
      described.set(description, cases);
    }
  }
}
if (described.size === 0) {
  console.error('No vendored description has conformance cases.');
  process.exit(1);
}

// A difference as both forms place it: the file and the 1-based line of its construct.
const native = (d) => ({ step: d.step, part: d.part, expected: d.expected, actual: d.actual,
  at: d.at ? `${basename(d.at.file)}:${d.at.line}` : null });
const wasm = (d) => ({ step: d.step, part: d.part, expected: d.expected, actual: d.actual,
  at: d.at ? `${basename(new URL(d.at.uri).pathname)}:${d.at.range.start.line + 1}` : null });

let failed = 0;
for (const [description, cases] of described) {
  let expected;
  try {
    expected = execFileSync(atd, ['validate', '--json', description], { encoding: 'utf8', maxBuffer: 1 << 28 });
  } catch (error) {
    // `atd validate` exits 1 when a case does not pass; its result is still on standard output.
    expected = error.stdout;
  }
  expected = JSON.parse(expected).cases.map((c) => ({ name: c.name, passed: c.passed,
    differences: c.differences.map(native) }));

  const server = WasmServer.withFiles(read, list);
  server.handle(JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { capabilities: {} } }));
  const [answer] = JSON.parse(server.handle(JSON.stringify({ jsonrpc: '2.0', id: 2, method: 'atd/validate',
    params: { root: pathToFileURL(description).href, cases } })));
  if (!answer.result) {
    console.error(`${description}: ${JSON.stringify(answer.error)}`);
    failed++;
    continue;
  }
  const actual = answer.result.cases.map((c) => ({ name: c.name, passed: c.passed,
    differences: c.differences.map(wasm) }));

  const name = (c) => c.name;
  expected.sort((a, b) => name(a).localeCompare(name(b)));
  actual.sort((a, b) => name(a).localeCompare(name(b)));
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    console.error(`${description}: atd/validate through WebAssembly differs from atd validate`);
    console.error(`  expected ${JSON.stringify(expected).slice(0, 2000)}`);
    console.error(`  got      ${JSON.stringify(actual).slice(0, 2000)}`);
    failed++;
  } else {
    const passed = actual.filter((c) => c.passed).length;
    console.log(`ok   ${description}: ${passed} of ${actual.length} case(s) passed, as atd validate says`);
  }
}
process.exit(failed === 0 ? 0 : 1);
