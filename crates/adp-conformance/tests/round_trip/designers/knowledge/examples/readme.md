# The Knowledge designer's examples

The files here are copied, byte for byte, from [`etalii-adp/etalii.adp`](https://github.com/etalii-adp/etalii.adp), folder `definitions/designers/examples/`, at commit `5ec503d2c25720416c866853a0741d38abe8a4fa` (etalii.adp #120, 2026-10-11, which added Peter's extra cities):

- `cities.yaml`, `cities.json` and `cities.xml`: one table, *Cities*, written in each of the three formats a knowledge file comes in;
- `LICENSE.md`: that repository's `LICENSE`, verbatim.

The three cities files no longer match that commit byte for byte: they carry the data the maintainer added to the showcase's `cities.yaml` and `provinces.yaml` on 2026-10-12 (ruling in chat that day, "Rewrite fixtures"), written in each format. The view *All cities* lists four columns, *Map* lists two, the table is left in *Map*, and Venlo relates to Limburg.

One file is not copied from there but written for ADP: `provinces.yaml`, the table the cities' relation *Province* names as its target. It exists so that the relation resolves: a city's province is shown by its name, not by the id the file holds. It is the showcase's `yaml/provinces.yaml`, with Limburg and Overijssel among its rows.

`.gitattributes` keeps this folder from line-ending conversion, because the module's tests read these bytes and compare them.

## What these examples do not demonstrate

- **No registration here.** The `.adp` file that makes a knowledge file a document of a project is in the showcase, `src/examples/designers/knowledge/`, where each format has a folder of its own with the data file and its registration.
- **No two-way relation and no parent relation.** `cities` relates to `provinces.yaml` one way; nothing in `provinces.yaml` shows the cities of a province, and no table nests its rows.
- **Nothing large.** Twelve rows or so say nothing about ten thousand.
- **No file with something wrong in it**: no unknown key, no value of the wrong type, no missing id. The module's tests write those themselves.
