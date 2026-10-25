# Assets

`materialdesignicons-subset.ttf` is the font file of [`@mdi/font`](https://www.npmjs.com/package/@mdi/font) 7.4.47, the version the standalone host's client pins, subset to the 37 glyphs the standalone host's Hierarchical Requirement Designer names at `a7fdf03` (etalii.adp spec 036, T026, research R7). `src/icons.rs` lists them, by name and code point. The fonts of the package are under the Apache License 2.0; its licence is `LICENSE.md`, verbatim from the package.

The glyphs are the `mdi-*` class names in `src/client/src/hierarchySurface/` and `src/designers/hierarchical-requirement/` of the standalone host at `a7fdf03`, and the icons `hierarchical-requirement.des` names. The subset is made with fontTools from the package's `fonts/materialdesignicons-webfont.ttf` (SHA-256 `61e8aba5a4e981fe22cf7c8e8bcdbea00476e75c62c37f01bf7ee33361d68428`, in `mdi-font-7.4.47.tgz`, SHA-256 `66131558352dc8df724feac30f3b7bf7ee7311a79e4f8436f3637c0993edb773`), the code points read from its `css/materialdesignicons.css`:

```bash
pyftsubset materialdesignicons-webfont.ttf --unicodes=<the code points of src/icons.rs> --output-file=materialdesignicons-subset.ttf --no-hinting --desubroutinize
```

A glyph added to the designer is added to `src/icons.rs` and the subset made again.
