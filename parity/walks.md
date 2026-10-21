# Walks

The manual walks of etalii.adp spec 030's user stories ([quickstart](https://github.com/etalii-adp/etalii.adp/blob/develop/specs/030-desktop-native-rust/quickstart.md), "Walk a user story by hand"): each one says what was done, what happened and whether it passed, per tool type. An inventory row whose Check is `walk: <story>/<step>` points at a step here. A difference from the standalone host's behaviour at [`8b6ccfc`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/8b6ccfca4f4a65e43c1a1a73a3224abe7ebf800e) is recorded as a failure, with what was done, what was expected and what happened; the walk fixes nothing, and a re-walk after the fixes records what each one changed beside the original findings.

## Story 2: work in a diagram with the same result on disk (task T062)

| | |
|---|---|
| Date | 2026-10-21 |
| Platform | Windows 11 Pro 10.0.26300, debug build of `adp-shell` (`target/debug/adp-desktop.exe`) at `0c1bd57`, dark theme, window 1600 by 1000 |
| Documents | copies of `parity/fixtures/freeplane/mindmap/example-1/` (opened by `mindmap.mm`), `parity/fixtures/generic/timeline/roadmap/` (opened by `roadmap.adp`) and `parity/fixtures/c4/container/courier/` (opened by `courier.containers.adp`), each opened as a project with `adp-desktop <folder>` |
| How | The real window, driven with Win32 input (`SetCursorPos`, `mouse_event`, `keybd_event`) and read through its AccessKit tree with UI Automation; screenshots with `PrintWindow`. Written bytes compared by SHA-256 against the untouched copy after every step, and `git diff --no-index` for what changed. Where the window could not be used (the timeline, see 2/3), the same input was given to the canvas in headless egui frames with the shell's canvas test harness (`crates/adp-shell/src/canvas/tests.rs`, a temporary test that was not kept), marked *headless* below. |
| Standalone behaviour | Read from the standalone host's source at `8b6ccfc` (`src/client/src/canvas/library/DiagramCanvas.tsx`, `src/client/src/canvas/gesture/usePointerGesture.ts`, the modules under `src/diagrams/`) and its specifications under `.spec-workflow/specs/` |

### Steps

| Step | What it covers | Scenario |
|---|---|---|
| 2/1 | Open a body and save it without edits (Ctrl+S): every file keeps its bytes | 1 |
| 2/2 | The same edits in both hosts write the same bodies and registrations | 2 |
| 2/3 | Pan, zoom, fit to view, box selection, Ctrl selection, drag, Escape cancels a drag, connect | 3 |
| 2/4 | A property changed on several selected elements changes all of them, and one undo reverts all of them | 4 |
| 2/5 | The body changed on disk while open: the canvas shows it, the person's edits are kept, and undo stops at the change from outside | 5 |
| 2/6 | Add from the toolbox by dragging an entry onto the canvas | FR-018 |
| 2/7 | Inline label rename (F2, double-click, the context menu's Rename) and Escape to cancel it | FR-018 |
| 2/8 | Add a child or a sibling from the keyboard, as the tool type's specification declares | FR-018 |
| 2/9 | Undo and redo from the toolbar and with Ctrl+Z and Ctrl+Y | FR-016 |
| 2/10 | Collapse and expand | FR-018 |

### Results

| Step | Mind map | Timeline | C4 container view |
|---|---|---|---|
| 2/1 | pass | pass | pass |
| 2/2 | pass (`adp-parity`) | pass (`adp-parity`) | pass (`adp-parity`) |
| 2/3 | **fail** (F1, F15, F2, F3) | **fail** (F1, F15, F5) | **fail** (F1, F15, F6) |
| 2/4 | pass, but see F4 | pass (*headless*) | pass |
| 2/5 | pass (F13) | pass (*headless*) | pass |
| 2/6 | **fail** (F7) | pass (*headless*) | **fail** (F8) |
| 2/7 | pass | pass (*headless*, F2 only) | **fail** (F9) |
| 2/8 | **fail** (F10, F11, F14) | not declared | not declared |
| 2/9 | pass | pass (*headless*) | pass |
| 2/10 | **fail** (F12) | not declared | not declared |

### What was done, per tool type

#### Mind map (`freeplane/mindmap`, `mindmap.mm`)

- **2/1** Opened `mindmap.mm` from the explorer, pressed Ctrl+S: `mindmap.mm`, `mindmap.adp` and `design.mm` unchanged. On opening, the view was at 72% with the right-hand branches cut off at the canvas edge; Fit to View then showed the whole map at 50% (F3).
- **2/3** Right-drag on empty canvas panned; a second right-drag with Escape before the release put the view back where that drag began. The plain mouse wheel panned (F1); Ctrl+wheel zoomed about the pointer (50% to 62%, then 75%); `-`, `=`, numpad `+` zoomed by 1.25 and `0` fitted (50%). A click selected *Scene graph*; Ctrl+click added *Rendering pipeline* (both outlined, the grid said "2 elements"). A left drag on empty canvas over the five constraint leaves selected the five ("5 elements"). Dragging *Overflow handling* onto *Persistence* did not re-parent it and wrote a `layout:` block into `mindmap.adp` (F2). A right drag from one node to another wrote nothing.
- **2/4** With *Scene graph* and *Rendering pipeline* selected, the grid showed Text and Notes empty (the values differ), Link, the read-only Collapsed and Identifier rows with their reasons, under VIEW and MODEL. Link set to `walk.md` wrote `LINK="walk.md"` on both nodes; one undo removed both and `mindmap.mm` was its original bytes again. The Enter that committed the field also added a sibling node "Node" (F4).
- **2/5** After renaming *Spacing* to *Gaps* on the canvas, `TEXT="Persistence"` was changed to `TEXT="Storage"` on disk by another program: within two seconds the canvas showed *Storage* and still *Gaps*, and the file held both. Ctrl+Z then said "The file was changed outside ADP, so the changes before that cannot be undone." in the status bar and changed nothing. The toolbar's Undo button refused too but showed no sentence anywhere (F13).
- **2/6** Dragging the toolbox's *Node* onto *Persistence* wrote nothing; the status bar said "Tool 'node' creates no node." (F7).
- **2/7** F2 on *Spacing*, Ctrl+A, "Gaps", Enter: `TEXT="Gaps"`. Double-click on *Anchoring*, "Anchors", Enter: `TEXT="Anchors"`. F2 on *Alignment*, typing, Escape: nothing written. Two undos restored the bytes.
- **2/8** Tab did nothing (the mind map declares Insert for a child and Enter for a sibling, not Tab). Insert on *Input handling* wrote a child "Node" and opened the inline editor, which was drawn over *Input handling* rather than the new child, while the canvas still showed the map without the child; typing "Walk child" and Enter renamed the new child, and only then did the canvas draw it (F10). The parent stayed selected (F11). Delete with five nodes selected removed only the last selected one (F14).
- **2/9** Undo and Redo on the toolbar and Ctrl+Z, Ctrl+Y undid and redid the grid edit, the added sibling, the rename and the drag one step at a time; after the last undo every file was its original bytes.
- **2/10** Space on *Constraints* did nothing, and its context menu offered Add child, Add sibling, Rename, Delete, Add notes and Link to, but no Collapse (F12).

#### Timeline (`generic/timeline`, `roadmap.adp`)

- **2/1** Opened by its registration `roadmap.adp`, Ctrl+S: `roadmap.adp` and `roadmap.tml` unchanged.
- **2/3** The canvas opened at 1% showing white bands and lines with no element or label in view; Fit to View, Zoom Out and `0` left it at 1% (F5). Right-drag panned and Escape put the view back (*headless*). At the zoom where one period fills the canvas (697%, set by hand in the harness), a click selected *Discovery*, but a 60-pixel drag wrote nothing, because 60 pixels are 8.6 seconds of a time axis that snaps to days (F5); Escape during a drag wrote nothing. Box selection over *Discovery* and *Private beta* selected both (given to the canvas as the box gesture's result, since the two cannot be on screen together); a connection from *Discovery* to *Private beta* (likewise) wrote one `relations` entry, and one undo removed it.
- **2/4** *Headless*: with *Discovery* and *Private beta* selected the grid showed Label, Begin, End and Row with empty shared values; Label set to "Shared" wrote both labels, one undo restored the bytes.
- **2/5** *Headless*: `label: Launch` changed on disk to `label: General availability`; the reconciled scene drew the new label, and undo was refused at the change from outside. `roadmap.adp` was never written.
- **2/6** *Headless*: the toolbox's *Element* dropped on the canvas wrote a new element "New element" on the nearest row and selected it; one undo restored the bytes.
- **2/7** *Headless*: F2 on *Discovery* opened the editor with "Discovery"; "Research" wrote `label: Research`; undo restored the bytes.

#### C4 container view (`c4/container`, `courier.containers.adp`)

- **2/1** Expanded `courier.dsl` in the explorer (the click on its arrow also opened `courier.dsl` itself, as `c4/context`, in a tab) and opened `courier.containers.adp`; Ctrl+S: all twelve files of the folder unchanged. The view opened fitted at 28% with the whole view in sight.
- **2/3** Zoom by keys as for the mind map. A click selected *Single-Page Application*, Ctrl+click added *Courier App* ("2 elements"); a left drag on empty canvas over both selected exactly those two. Dragging *Event Broker* wrote its position to `courier.layout.json`, but the box stayed where the view's `autoLayout lr` puts it, also after closing and reopening the tab: the standalone host's `C4Layout` likewise lets `autoLayout` win over authored positions. Dragging the two selected containers wrote both positions in one edit, and one undo took both out; a drag with Escape before the release wrote nothing. A right drag from *Courier App* to *Event Broker* wrote nothing and said nothing (F6).
- **2/4** With *Single-Page Application* and *Courier App* selected, the grid showed Name, Description and Technology empty (they differ), Tags read-only "until an action exists for them", Kind and Identifier read-only under MODEL. Technology set to "Rust" rewrote the technology argument of both lines of `courier.dsl`; one undo restored the bytes, redo wrote them again, undo restored them.
- **2/5** After renaming *Courier App* to *Courier Mobile* in the grid, `container "Event Broker"` was changed to `container "Message Bus"` on disk: the canvas showed *Message Bus* and *Courier Mobile*, the file held both, and Ctrl+Z was refused with the outside-change sentence.
- **2/6** Dragging *Container* from the toolbox onto empty canvas inside the *Courier Tracking* boundary asked for a name in an "Add container" dialog, then wrote nothing and said "A container lives inside a software system. Drop it on one." (F8). Dragging *Person* onto empty canvas outside the boundary asked for a name, wrote `walkVisitor = person "Walk Visitor" "Describe Walk Visitor."` into the model, and the view drew it; one undo restored `courier.dsl`. The refusal sentence of the earlier drop stayed in the status bar after this success.
- **2/7** F2 on *Courier App* and a double-click on it opened the inline editor empty, and Enter wrote nothing: "This diagram does not offer that here." (F9). The context menu's *Rename…* opened the editor with "Courier App", and "Courier Phone" was written.
- **2/9** As for the mind map. Every move and undo rewrites `courier.layout.json` whole, in CRLF (the fixture is LF), so after a move and its undo the sidecar holds the same values in other bytes; the goldens show the standalone host writes the sidecar the same way.

### Failures and differences

| # | Tool types | What was done | Expected (standalone host at `8b6ccfc`) | What happened |
|---|---|---|---|---|
| F1 | all | Turned the mouse wheel over the canvas, without Ctrl | The wheel zooms about the pointer by a step of 1.25 per notch (`DiagramCanvas.tsx`, `onWheel`) | The wheel pans the view; only Ctrl+wheel zooms, by a varying factor (1.25, then 1.2) |
| F15 | all | Dragged with the left button on empty canvas; dragged with the right button on empty canvas | A left drag on the background pans (`usePointerGesture`'s background target, `case "background"`); the right button opens the context menu, and draws a relation only where a definition sets `connectOnRightDrag` | A left drag draws a selection box; a right drag pans. The standalone host has no box selection or Ctrl+click on the canvas at `8b6ccfc` (the `multi-select` specification has no tasks there), so spec 030's scenario 3 and the standalone host's gestures disagree; its Requirement 3.4 asks that the gesture that pans keeps panning |
| F2 | mind map | Dragged *Overflow handling* onto *Persistence* | The node becomes *Persistence*'s last child (`MindmapCanvas.tsx`, `moveElement`; the specification: "dropping it on another node makes it that node's last child … The drag never writes a position") | Nothing moved in `mindmap.mm`; `mindmap.adp` gained `layout:` with `ID_0jkyc0uyotnmbcen585kev647: 172.476 326.315`, bytes the standalone host never writes |
| F3 | mind map | Opened `mindmap.mm` | The view fits the whole map | The view opened at 72% with the right-hand branches cut off; Fit to View then gave 50% with everything in view |
| F4 | mind map | With two nodes selected, typed `walk.md` into the grid's Link field and pressed Enter | Enter commits the field only | The link was written to both nodes and the same Enter reached the canvas as the Add sibling shortcut: a sibling "Node" was written after *Rendering pipeline* |
| F5 | timeline | Opened `roadmap.adp`; pressed Fit to View, Zoom Out, `0`; dragged a period | The canvas scales seconds to units (`timelineScaleOf`: the span over the width; the specification's `scale` of 20 units a day) and fits the roadmap with its ruler; a drag moves a period by days | The time axis is drawn in raw seconds (the roadmap spans 108,864,139 units), so fit stops at the 100,000-unit view limit (1%): white bands, no element, no label, no ruler. Periods are 120 units wide whatever their length, and a drag must cover a whole day of seconds (600,000 pixels at the zoom where one period is readable) to write anything |
| F6 | C4 | Right-dragged from *Courier App* to *Event Broker*; looked for anchors on an element's edge | Elements have edge anchors (`C4Canvas.tsx`, `anchors: { kind: "edge" }`) and a drag from one to another element draws a relationship and opens its description | No anchors are drawn, and the right drag writes nothing and says nothing (the canvas returns early for a module-drawn diagram, which has no compiled notation) |
| F7 | mind map | Dragged the toolbox's *Node* onto *Persistence* | A child is added under *Persistence* ("Drop on a node to add a child under it.") | Nothing written; "Tool 'node' creates no node." |
| F8 | C4 | Dragged *Container* onto the *Courier Tracking* boundary of the container view | The boundary is an element of the canvas (`C4Canvas.tsx` puts the boundaries among the elements), so the drop lands on the system and adds the container to it | After the name was asked, nothing written: "A container lives inside a software system. Drop it on one." The boundary is not a drop target, and the view has no other software system to drop on |
| F9 | C4 | F2 on a container; double-click on it | F2 is the Rename action's shortcut (`C4ContextActionProvider`), opening the editor with the name | The canvas's label editor opens empty, and committing it is refused: "This diagram does not offer that here." Only the context menu's Rename… works |
| F10 | mind map | Insert on *Input handling* (add child) | The new child is drawn and its editor opens over it | The child is written, but the canvas is not redrawn until the editor is committed or another selection is made, and the editor is drawn over the parent; the same after Enter (add sibling) |
| F11 | mind map | Insert, then committed the name | The new child is selected (edit-on-drop selects what it created) | The parent stays selected |
| F12 | mind map | Space on a node with children; its context menu | Space and the menu's view group collapse or expand (the specification's `toggleFold`, shortcut Space) | Space does nothing (the canvas only routes Tab, Enter, Delete, Backspace and Insert to declared shortcuts), and the menu has no Collapse or Expand |
| F13 | all | Pressed the toolbar's Undo after a change on disk | The refusal is said | Nothing is shown; Ctrl+Z shows the sentence in the status bar |
| F14 | mind map | Selected five nodes with a box and pressed Delete | Every selected node is deleted, or nothing (the multi-select specification's Requirement 7.3) | Only the last selected node was deleted |

Also seen, outside Story 2: the click on `courier.dsl`'s expander arrow in the explorer opened `courier.dsl` in a tab as well (Story 1).

### Re-walk after the fixes

| | |
|---|---|
| Date | 2026-10-21 |
| Platform | As above, a debug build of `adp-shell` at `6a9e77e` (branch `features/030-walk-fixes`), on fresh copies of the same three folders |
| How | As above: the real window driven with Win32 input and read through its AccessKit tree, the written bytes compared by SHA-256 and `git diff --no-index` after every step. Each fix also has a headless test that fails at `c1d708f` and passes after it (`crates/adp-shell/src/canvas/walk_tests.rs`, `crates/adp-shell/src/explorer.rs`, `modules/diagrams/timeline/tests/session.rs`) |
| Standalone behaviour | Read again at `8b6ccfc` for each finding before it was fixed; where the source disagreed with what the walk expected, the source decided (F6, F8, F9's double-click) |

| # | Fixed | Test | Re-walked in the window |
|---|---|---|---|
| F1 | yes: a wheel notch zooms by 1.25 about the pointer, with or without Ctrl; a pinch by its own factor | `f1_the_plain_wheel_zooms_about_the_pointer_by_one_step_a_notch` | Mind map: one notch up over *Scene graph* 50% to 63% with *Scene graph* still under the pointer, two down to 40%, Ctrl and one up to 50% |
| F2 | yes: a drag of a node the layout places proposes a parent: dropped with its centre on another node it becomes that node's last child, one undo away; on empty canvas or its own branch the drag ends; no position is written | `f2_a_node_dragged_onto_another_becomes_its_last_child_and_no_position_is_written` | *Overflow handling* dragged onto *Persistence*: its `<node>` moved to the end of *Persistence*'s, `mindmap.adp` unchanged; Ctrl+Z gave back every byte |
| F3 | yes: the view follows the fit, as the standalone host's `null` view does, until the person zooms, pans or scrolls; Fit to View follows it again | `f3_the_view_stays_fitted_as_the_surface_changes_until_the_person_changes_it` | `mindmap.mm` opened at 50% with the whole map in view; `courier.containers.adp` at 27%, `roadmap.adp` at 61% |
| F4 | yes: the canvas's keys are its own only while it has the keyboard (it is handed it when its tab first draws, and by a press on it) | `f4_enter_in_a_field_of_the_property_grid_commits_the_field_and_never_reaches_the_canvas` | *Scene graph* and *Rendering pipeline* selected, Link set to `walk.md` with Enter: `LINK="walk.md"` on both and no sibling; Ctrl+Z gave back every byte |
| F5 | yes: a time axis of instants is scaled once, as `timelineScaleOf` scales it (the span with a tenth to spare either side across 1200 units; an empty timeline 20 units a day), the module's 60 units a row kept; a period is as long as it lasts; its edge writes its begin or its end; the ruler `timelineTicks.ts` labels runs along the bottom | `f5_a_timeline_opens_fitted_in_readable_units_with_its_ruler_and_its_periods_as_long_as_they_last`, `f5_a_period_dragged_by_days_moves_by_days_and_its_edge_resizes_it`, `f5_a_connection_from_a_period_released_on_empty_canvas_makes_its_target_at_the_date_under_it`, `a_drag_of_a_periods_edge_writes_its_begin_or_its_end` | `roadmap.adp` opened fitted at 61%, every element and label in view, the ruler reading 2025 to 2028; a 40-pixel drag of *Private beta* wrote `begin: 2025-11-22` and `end: 2026-01-06` (its 45 days kept); its right edge dragged 30 pixels wrote `end: 2025-12-19` alone; `0` fitted at 61% again; each undone to the original bytes |
| F6 | as the standalone host: each card has an anchor in the middle of each edge, drawn when it is selected or under the pointer, and a drag from one to another card writes nothing, now saying "This diagram does not offer that here." The walk expected a relationship, but at `8b6ccfc` `C4Canvas.tsx` hands no `onConnectionDrawn` to `DiagramCanvas` (whose `dispatchDiagramEvent` drops an event without a handler) and the C4 module has no command that adds a relationship, so writing one would write bytes the standalone host never writes. Left for a decision | `f6_a_c4_card_has_an_anchor_on_each_edge_and_a_drag_from_one_writes_nothing_and_says_why` | Anchors drawn on *Courier App*; a drag from its right anchor to *Single-Page Application* wrote nothing and said the sentence |
| F7 | yes: a toolbox tool that runs an operation runs it on the node it is dropped on, as that node's menu runs it; over empty canvas nothing happens (`MindmapCanvas.tsx`, `onElementDropped`) | `f7_the_toolbox_node_dropped_on_a_node_adds_a_child_under_it` | *Node* dropped on *Persistence*: a child "Node" written under it, drawn, selected and named in place; "Dropped child" written; two undos gave back every byte |
| F8 | not changed: the standalone host refuses this drop too. `C4Canvas.tsx`'s `onElementDropped` looks for the target among `model.nodes` only, never the boundaries, which are `selectable: false` "read, never picked", so a drop inside a boundary goes to the module with no parent and its `C4Placement` refuses it with the same sentence | none (behaviour kept) | *Container* dropped inside the *Courier Tracking* boundary, named "Walk Store": nothing written, "A container lives inside a software system. Drop it on one." |
| F9 | yes for F2: F2 runs the shortcut the menu declares on it before the label's own editor, so on a C4 card it is the module's Rename…, opened in place with the name. Not for the double-click: `C4Canvas.tsx` declares no `activate` gesture, so a double-click on a card does nothing in the standalone host; it now does nothing here instead of opening an empty editor that refused its commit | `f9_f2_on_a_c4_element_renames_it_in_place_through_the_modules_rename_and_a_double_click_does_nothing` | F2 on *Courier App* opened the editor with "Courier App"; "Courier Phone" rewrote the name in its `container` line, Ctrl+Z gave back every byte; a double-click opened nothing |
| F10 | yes: a node an action adds before its name is asked is drawn in the same frame and the editor opens over it | `f10_f11_a_node_added_by_key_is_drawn_at_once_selected_and_named_in_place_over_itself` | Insert on *Input handling*: the child drawn at once with the editor over it; "Walk child" written. Enter on it: a sibling "Node" after it, drawn, the editor over it |
| F11 | yes: that node is selected | as F10 | The properties pane named the new child; it was outlined |
| F12 | yes: the module binds this viewer's folds to `self.view`, so the menu offers Collapse or Expand, and the canvas routes Space, which the specification declares; what a fold hides is not drawn; nothing is written or put on the history | `f12_space_and_the_menu_collapse_and_expand_a_branch_for_this_viewer_only` | The menu of *Constraints* offered Collapse, which hid its five leaves and drew the collapsed glyph; Space on it expanded and collapsed it; `mindmap.mm` unchanged |
| F13 | yes: the toolbar's Undo and Redo say their refusal as Ctrl+Z does and ask for the frame that shows it | `f13_the_toolbar_undo_says_why_it_was_refused_and_a_later_success_takes_it_away` | After *Spacing* renamed to *Gaps* and `TEXT="Persistence"` changed to `TEXT="Storage"` on disk, the toolbar's Undo put "The file was changed outside ADP, so the changes before that cannot be undone." in the status bar without the pointer moving |
| F14 | yes: a deletion with several selected deletes all of them as one edit, asking first where the specification asks before any of them, or none of them with the reason | `f14_delete_with_several_selected_deletes_all_of_them_as_one_undo_or_none` | The five constraint leaves boxed ("5 elements") and Delete: all five gone, one Ctrl+Z gave back every byte |
| F15 | not changed: waits for a decision | | |

Also fixed: an edit that goes through takes a refusal off the status bar (walked: after the refused Undo above, renaming *Alignment* cleared it, and a second refused Undo said it again); a press on an explorer row's chevron expands it without opening the file (`a_press_on_the_chevron_of_a_file_with_registrations_expands_it_without_opening_it`; walked: `courier.dsl`'s chevron expanded its five registrations and opened no tab); and a connection released on empty canvas reads the point through the module, so the timeline makes its target at the date under it instead of refusing the number its time functions cannot read (found while testing F5).

Seen on the re-walk and not fixed: a double-click on a mind map node still renames it, although `mindmap.dis` says `doubleClick: "none"` (the standalone host opens nothing); the properties pane names a single selected mind map node by its id rather than its text; the collapsed glyph `⊕` is drawn as a box (the font has no such glyph); and the walk's own input sometimes loses the first press after it brings the window forward, so a first click or toolbox drag was repeated.

| Step | Mind map | Timeline | C4 container view |
|---|---|---|---|
| 2/3 | pass, but see F15 | pass, but see F15 (now in the window) | as the standalone host (F6), but see F15 |
| 2/4 | pass | pass (*headless*, not re-walked) | pass (not re-walked) |
| 2/5 | pass | pass (*headless*, not re-walked) | pass (not re-walked) |
| 2/6 | pass | pass (*headless*, not re-walked) | as the standalone host (F8) |
| 2/7 | pass (not re-walked) | pass (*headless*, not re-walked) | pass |
| 2/8 | pass | not declared | not declared |
| 2/9 | pass | pass (*headless*, not re-walked) | pass |
| 2/10 | pass | not declared | not declared |

### Screenshots

- [Mind map fitted to view](walks/story2-mindmap-fit-to-view.png)
- [Mind map, Ctrl selection](walks/story2-mindmap-ctrl-select.png)
- [Mind map, the Enter of the grid added a sibling (F4)](walks/story2-mindmap-enter-leak.png)
- [Mind map, change on disk drawn and undo refused](walks/story2-mindmap-disk-change-barrier.png)
- [Mind map, toolbox drop refused (F7)](walks/story2-mindmap-toolbox-refused.png)
- [Timeline as it opens (F5)](walks/story2-timeline-open.png)
- [C4 container view, two elements in the grid](walks/story2-c4-two-elements-grid.png)
- [C4, F2 opens an empty editor (F9)](walks/story2-c4-inline-rename-empty.png)
- [C4, change on disk drawn with the person's edit](walks/story2-c4-disk-change.png)
- [Re-walk: the timeline as it opens, fitted, with its ruler (F5)](walks/story2-rewalk-timeline-open.png)
- [Re-walk: Insert draws the child and names it in place over itself (F10, F11)](walks/story2-rewalk-mindmap-insert.png)
- [Re-walk: *Constraints* collapsed from its menu (F12)](walks/story2-rewalk-mindmap-collapsed.png)
