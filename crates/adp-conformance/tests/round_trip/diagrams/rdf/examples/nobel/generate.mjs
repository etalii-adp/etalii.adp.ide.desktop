// The derivation this folder's readme documents: fetches the Nobel Prize API's CC0 data and
// writes the two vendored Turtle files beside this script. Run with `node generate.mjs` and
// commit what it writes; the readme records when it last ran.
//
// The API is JSON; the historical linked-data endpoints at data.nobelprize.org now serve an
// application shell, so the Turtle here is derived rather than copied - which CC0 permits
// without condition, and this script is the exact derivation.

const API = "https://api.nobelprize.org/2.1";

const fetchJson = async (url) => {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${url} answered ${response.status}`);
  }
  return response.json();
};

const escape = (value) =>
  String(value).replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\r?\n/g, "\\n");

const slug = (value) =>
  String(value).toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");

const header =
  "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\r\n" +
  "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\r\n" +
  "@prefix nobel: <https://data.nobelprize.org/terms/> .\r\n" +
  "@prefix laureate: <https://data.nobelprize.org/resource/laureate/> .\r\n" +
  "@prefix prize: <https://data.nobelprize.org/resource/nobelprize/> .\r\n" +
  "@prefix category: <https://data.nobelprize.org/resource/category/> .\r\n" +
  "\r\n";

const laureates = [];
for (let offset = 0; ; offset += 1000) {
  const page = await fetchJson(`${API}/laureates?limit=1000&offset=${offset}`);
  laureates.push(...(page.laureates ?? []));
  if (laureates.length >= (page.meta?.count ?? laureates.length)) {
    break;
  }
}
laureates.sort((a, b) => Number(a.id) - Number(b.id));

const nameOf = (laureate) => laureate.knownName?.en ?? laureate.orgName?.en ?? `Laureate ${laureate.id}`;

// Every prize any laureate holds, keyed category-year, in stable order.
const prizes = new Map();
for (const laureate of laureates) {
  for (const prize of laureate.nobelPrizes ?? []) {
    const key = `${slug(prize.category.en)}-${prize.awardYear}`;
    if (!prizes.has(key)) {
      prizes.set(key, { key, category: prize.category.en, year: prize.awardYear });
    }
  }
}

const categories = [...new Set([...prizes.values()].map((prize) => prize.category))].sort();

const categoryBlock = (category) =>
  `category:${slug(category)} a nobel:Category ;\r\n    rdfs:label "${escape(category)}" .\r\n\r\n`;

const prizeBlock = (prize) =>
  `prize:${prize.key} a nobel:NobelPrize ;\r\n` +
  `    rdfs:label "Nobel Prize in ${escape(prize.category)} ${prize.year}" ;\r\n` +
  `    nobel:year "${prize.year}"^^xsd:gYear ;\r\n` +
  `    nobel:category category:${slug(prize.category)} .\r\n\r\n`;

const laureateBlock = (laureate, withMotivation) => {
  const lines = [
    `laureate:${laureate.id} a nobel:Laureate ;`,
    `    rdfs:label "${escape(nameOf(laureate))}" ;`,
  ];
  if (laureate.birth?.date && laureate.birth.date !== "0000-00-00") {
    lines.push(`    nobel:birthDate "${laureate.birth.date}"^^xsd:date ;`);
  }
  for (const prize of laureate.nobelPrizes ?? []) {
    lines.push(`    nobel:nobelPrize prize:${slug(prize.category.en)}-${prize.awardYear} ;`);
    if (withMotivation && prize.motivation?.en) {
      lines.push(`    nobel:motivation "${escape(prize.motivation.en)}"@en ;`);
    }
  }
  const last = lines.pop().replace(/ ;$/, " .");
  return lines.join("\r\n") + "\r\n" + last + "\r\n\r\n";
};

// laureates.ttl: the whole dataset - deliberately larger than the drawn-element budget, so the
// truncated first-N view has a real file to be honest about.
let all = header;
for (const category of categories) {
  all += categoryBlock(category);
}
for (const prize of [...prizes.values()].sort((a, b) => a.key.localeCompare(b.key))) {
  all += prizeBlock(prize);
}
for (const laureate of laureates) {
  all += laureateBlock(laureate, false);
}

// nobel-1903.ttl: one coherent year - the year both Curies stood in Stockholm - small enough to
// read whole, with the motivations as literal rows.
const YEAR = "1903";
const year = [...prizes.values()].filter((prize) => prize.year === YEAR);
const yearLaureates = laureates.filter((laureate) =>
  (laureate.nobelPrizes ?? []).some((prize) => prize.awardYear === YEAR));
let small = header;
for (const category of [...new Set(year.map((prize) => prize.category))].sort()) {
  small += categoryBlock(category);
}
for (const prize of year.sort((a, b) => a.key.localeCompare(b.key))) {
  small += prizeBlock(prize);
}
for (const laureate of yearLaureates) {
  small += laureateBlock(
    { ...laureate, nobelPrizes: laureate.nobelPrizes.filter((prize) => prize.awardYear === YEAR) },
    true);
}

const fs = await import("node:fs");
fs.writeFileSync(new URL("./laureates.ttl", import.meta.url), all);
fs.writeFileSync(new URL("./nobel-1903.ttl", import.meta.url), small);
console.log(`laureates.ttl: ${laureates.length} laureates, ${prizes.size} prizes, ${categories.length} categories`);
console.log(`nobel-1903.ttl: ${yearLaureates.length} laureates, ${year.length} prizes`);
