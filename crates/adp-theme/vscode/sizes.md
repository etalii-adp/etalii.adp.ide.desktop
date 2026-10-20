# Sizes of the workbench

The sizes VS Code's workbench draws with, which its theme files do not hold (etalii.adp spec 030, research R9, contracts/vscode-theme.md). Each value is taken from microsoft/vscode at the tag in [SOURCE.md](SOURCE.md), `1.142.0`, at the file and line given. `crates/adp-theme/tests/vscode_values.rs` compares every row with the `Sizes` the shell draws with.

| Size | Value | Source |
|---|---|---|
| workbench_font | 13 | `src/vs/workbench/browser/media/style.css:49`, `.monaco-workbench { font-size: 13px }` |
| editor_font | 14 | `src/vs/editor/common/config/fontInfo.ts:239`, `EDITOR_FONT_DEFAULTS.fontSize`, 14 on Windows and Linux |
| editor_font_macos | 12 | `src/vs/editor/common/config/fontInfo.ts:240`, `EDITOR_FONT_DEFAULTS.fontSize`, 12 on macOS |
| row_height | 22 | `src/vs/workbench/browser/parts/views/treeView.ts:1246`, `TreeRenderer.ITEM_HEIGHT` |
| tab_height | 35 | `src/vs/workbench/browser/parts/editor/editorTabsControl.ts:107`, `EDITOR_TAB_HEIGHT.normal` |
| title_bar_height | 35 | `src/vs/platform/window/common/window.ts:386`, `DEFAULT_CUSTOM_TITLEBAR_HEIGHT` |
| status_bar_height | 22 | `src/vs/workbench/browser/parts/statusbar/statusbarPart.ts:124`, `StatusbarPart.HEIGHT` |
| activity_bar_width | 48 | `src/vs/workbench/browser/parts/activitybar/activitybarPart.ts:50`, `ACTIVITYBAR_WIDTH` |
| focus_outline_width | 1 | `src/vs/workbench/browser/media/style.css:273`, `outline-width: 1px` |
| border_width | 1 | `src/vs/workbench/browser/media/style.css:311`, `outline: 1px solid` |
