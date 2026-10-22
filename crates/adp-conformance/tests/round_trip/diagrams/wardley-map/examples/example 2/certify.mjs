// Certifies the fixtures against the real OnlineWardleyMaps parser, so a file in this folder
// is proven to be real DSL rather than ADP's idea of the DSL. This is the counterpart of the
// C4 corpus being certified by the Structurizr CLI.
//
// The parser is the one vendored by `cli-owm` (MIT, Copyright 2019 Damon Skelhorn). Install it
// somewhere and point at it:
//
//   npm install --no-save cli-owm@0.0.2
//   node certify.mjs *.owm
//
// or, if it is installed elsewhere:
//
//   CLI_OWM=/path/to/node_modules/cli-owm/dist/index.js node certify.mjs *.owm
//
// Exit code is non-zero when any file reports a parse error, so this can gate CI later.
// Set CERTIFY_DUMP=1 to print each parsed component, which is how the decorator-versus-keyword
// question recorded in readme.md was settled.
import { readFileSync, existsSync } from "node:fs";
import { pathToFileURL } from "node:url";

// A path needs converting to a file:// URL before import; a bare package name is passed through.
const configured = process.env.CLI_OWM ?? "cli-owm";
const specifier = existsSync(configured) ? pathToFileURL(configured).href : configured;
const { parse } = await import(specifier);

let failed = false;

for (const file of process.argv.slice(2)) {
  const text = readFileSync(file, "utf8");

  let map;
  try {
    map = parse(text);
  } catch (error) {
    console.log(`${file}\n  PARSE THREW - ${error.message}`);
    failed = true;
    continue;
  }

  const counts = {};
  for (const [key, value] of Object.entries(map)) {
    if (Array.isArray(value) && value.length > 0) {
      counts[key] = value.length;
    }
  }

  const errors = map.errors ?? [];
  console.log(`${file}\n  parsed: ${JSON.stringify(counts)}\n  errors: ${errors.length}`);

  for (const error of errors) {
    // `line` is 0-based in this parser, which is worth knowing before comparing it against
    // DiagramProblemLineLocation, whose Number is 1-based.
    console.log(`    ! ${JSON.stringify(error)}`);
  }

  if (errors.length > 0) {
    failed = true;
  }

  if (process.env.CERTIFY_DUMP) {
    for (const component of map.components ?? []) {
      const set = Object.fromEntries(
        Object.entries(component).filter(
          ([, v]) => v !== undefined && v !== null && v !== false && v !== "",
        ),
      );
      console.log(`    - ${JSON.stringify(set)}`);
    }
  }
}

process.exit(failed ? 1 : 0);
