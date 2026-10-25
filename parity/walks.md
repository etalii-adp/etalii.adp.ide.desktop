# Walks

The manual walks of etalii.adp spec 030's user stories ([quickstart](https://github.com/etalii-adp/etalii.adp/blob/develop/specs/030-desktop-native-rust/quickstart.md), "Walk a user story by hand"): each one says what was done, what happened and whether it passed, per tool type. An inventory row whose Check is `walk: <story>/<step>` points at a step here. A difference from the standalone host's behaviour at [`8b6ccfc`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/8b6ccfca4f4a65e43c1a1a73a3224abe7ebf800e) is recorded as a failure, with what was done, what was expected and what happened; the walk fixes nothing, and a re-walk after the fixes records what each one changed beside the original findings.

## Story 1: install, open a workspace and find one's way in it (task T037)

| | |
|---|---|
| Date | 2026-10-22 |
| Walked by | The maintainer, by hand, on the pre-release builds of `develop` from the repository's releases |
| Steps | Start the program; add a folder and restart; change a file in it outside the program; use the explorer by the keyboard alone (the arrows, F2, Delete, Shift+F10); open, edit and save a Markdown, a JSON, an XML and a YAML file |
| Result | **pass**, as the maintainer reported it on 2026-10-22; the platforms and the step-by-step findings were not recorded |

## Story 1 with a screen reader, and Story 6 (task T089; SC-009, SC-010)

| | |
|---|---|
| Date | 2026-10-22 |
| Walked by | The maintainer, by hand |
| Steps | Story 1's steps with a screen reader and without sighted help; the window beside VS Code 1.142 in Dark 2026 and in its default light theme, comparing the explorer, the tabs, the panels, the menus and the status bar |
| Result | **pass**, as the maintainer reported it on 2026-10-22; the platforms and the screen reader used were not recorded |

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
| F15 | yes, by Peter's decision on 2026-10-21: pan, as the standalone host. A left drag on empty canvas pans (`usePointerGesture.ts`, `case "background"`) and Escape puts the view back where the drag began; there is no selection box. The right button is the menu's: a right click opens the context menu of what it is on, and a right drag draws a relation from the element it starts on only where the specification binds the connect gesture to the right button from the body (`connectOnRightDrag`: the causal loop diagram and Agent Behavior Modelling by their `connect.pointer`, the functional decomposition graph, Sankey and supply chain modules by their own word), and anywhere else does nothing. Ctrl+click still adds to and takes from the selection, which the property grid on several elements needs (Story 2 scenario 4, P-031) | `a_left_drag_on_empty_canvas_pans_keeps_the_selection_and_escape_puts_the_view_back`, `a_right_click_opens_the_menu_and_a_right_drag_does_nothing_where_the_specification_draws_no_relation_with_it`, `a_right_drag_from_one_variable_to_another_draws_a_link_where_the_specification_binds_it_to_the_right_button`, `a_right_drag_from_one_element_body_to_another_draws_the_relation_the_pair_admits`, and the arbiter's own tests in `crates/adp-shell/src/canvas/gestures/mod.rs` | not re-walked |

Also fixed: an edit that goes through takes a refusal off the status bar (walked: after the refused Undo above, renaming *Alignment* cleared it, and a second refused Undo said it again); a press on an explorer row's chevron expands it without opening the file (`a_press_on_the_chevron_of_a_file_with_registrations_expands_it_without_opening_it`; walked: `courier.dsl`'s chevron expanded its five registrations and opened no tab); and a connection released on empty canvas reads the point through the module, so the timeline makes its target at the date under it instead of refusing the number its time functions cannot read (found while testing F5).

Seen on the re-walk and not fixed: a double-click on a mind map node still renames it, although `mindmap.dis` says `doubleClick: "none"` (the standalone host opens nothing; fixed since: a double-click runs the entry each notation's `doubleClick` names, and nothing for `none`, `f9_a_double_click_on_a_mind_map_node_starts_no_rename_and_f2_still_renames_it`, not re-walked); the properties pane names a single selected mind map node by its id rather than its text; the collapsed glyph `⊕` is drawn as a box (the font has no such glyph); and the walk's own input sometimes loses the first press after it brings the window forward, so a first click or toolbox drag was repeated.

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

## Story 4: work in the Knowledge designer (task T067)

| | |
|---|---|
| Date | 2026-10-21 |
| Platform | Windows 11 Pro 10.0.26300, debug build of `adp-shell` (`target/debug/adp-desktop.exe`) at `fcd85dd` for 4/1 to 4/8, release build (`target/release/adp-desktop.exe`) at the same commit for 4/9, dark theme, window 1600 by 1000 at 125% |
| Documents | copies of `parity/fixtures/etalii/knowledge/cities-yaml/`, `cities-json/` and `cities-xml/` (each `cities.<format>`, its registration `cities.adp` and `provinces.yaml`, the table its relation names), each opened as a project with `adp-desktop <folder>`; and a generated `places.yaml` of 10,000 rows (1,752,144 bytes), made by the same rule as `benches/table.rs` (a name, a number, one of three sizes, a tick on every other row) |
| How | The real window, driven with Win32 input and read through its AccessKit tree with UI Automation, screenshots with `PrintWindow`, as for Story 2. The edits of the parity script `parity/scripts/etalii/knowledge/cities-<format>.json` were made by hand in the same order, as far as the window offers them, and every written file was hashed (SHA-256) after every step. The ids the desktop host generated (the new option and the new row) were replaced by the ids `recorded.json` gives for the same steps before the result was compared byte for byte with the golden. The tick, date and time edits were made in the property grid, because the view the script edits in (*Map*) does not show those columns until the table is scrolled sideways (4/2). The memory of 4/9 is the process's private bytes, sampled while the scroll bar was dragged; its frame times are those of the headless benchmark `cargo test -p adp-shell --release --test table`. Nothing was checked headless in this walk. |
| Standalone behaviour | Read from the standalone host's source at `8b6ccfc` (`src/client/src/table/library/`: `TableSurface.tsx`, `header/ColumnHeader.tsx`, `header/columnActions.ts`, `rows/CalculationRow.tsx`, `views/ViewTabs.tsx`, `views/GroupEditor.tsx`; `src/designers/knowledge/client/knowledgeDefinition.ts`; `src/designers/knowledge/backend/EtAlii.Adp.Designer.Knowledge/KnowledgeRowContext.cs` and `KnowledgeEdits.cs`) and the goldens under `parity/goldens/etalii/knowledge/` |

A walk of Story 2 ran in another window on the same desktop at the same time. Its driver took the foreground now and then, and a few presses of this walk were lost to it (a press on a list entry, a double-click, the first press on a tab); each was made again once the file's hash showed nothing had been written, and nothing below rests on a press that was not checked against the file.

### Steps

| Step | What it covers | Scenario |
|---|---|---|
| 4/1 | Open the data file, and the registration, from the explorer, and save with Ctrl+S without edits: every file keeps its bytes | 1 |
| 4/2 | Edit a cell of every kind: text and number in the table, tick, date and time in the property grid, one option, several options, a new option made from the list, a relation; a number that is refused | 1 |
| 4/3 | Add a row with *+ New* and fill it in | 1 |
| 4/4 | Change views: show the other view, add a sort and turn one, add a filter condition and give it a comparison and a value, hide empty groups, fold a group, hide a column, widen a column, add a view and rename it | 1 |
| 4/5 | Undo and redo with Ctrl+Z and Ctrl+Y: each restores the bytes of the step before | 1 |
| 4/6 | The written body after the whole sequence, byte for byte against the standalone host's golden | 1, FR-022 |
| 4/7 | Relations between files: a relation chosen from the other table's rows, a related row renamed in the other table's tab, a related row deleted (the question), the related file changed on disk | FR-022 |
| 4/8 | The body changed on disk while open: the table shows it, the person's edits are kept, and undo stops at the change from outside | 1 |
| 4/9 | A table of 10,000 rows scrolled from top to bottom: no visible stall, and memory that does not grow with each screen | 2, SC-006 |

### Results

| Step | YAML (`cities.yaml`) | JSON (`cities.json`) | XML (`cities.xml`) |
|---|---|---|---|
| 4/1 | pass | pass | pass |
| 4/2 | pass | pass | pass |
| 4/3 | pass | pass | pass |
| 4/4 | **fail** (K1, K2, K6) | **fail** (K1, K2, K6) | **fail** (K1, K2, K6) |
| 4/5 | pass, but see K3 | pass | pass |
| 4/6 | **fail** (K1; one more line explained by the walk's order) | **fail** (K1; the same) | **fail** (K1 only) |
| 4/7 | pass, but see K5 | not walked | not walked |
| 4/8 | pass | not walked | not walked |
| 4/9 | pass (`places.yaml`) | | |

Seen in every format, outside a step: K4 (the first column when the table scrolls sideways), K7 (no line of calculations), K8 (an unticked checkbox has no box), K9 (the wheel's step).

### What was done

#### 4/1 to 4/6, the script's sequence in each format

- **4/1** Opening `cities.<format>` from the explorer opened the table in its *Map* view (the file's `activeView: v2`), with the registration `cities.adp` nested under it; Ctrl+S left `cities.<format>`, `cities.adp` and `provinces.yaml` unchanged. Opening `cities.adp` from the explorer showed the tab of its body and said "Opened cities.adp as etalii/knowledge".
- **4/2** Double-click on *Amsterdam*, Ctrl+A, "Amsterdam (NH)", Enter: the text written. A press on Antwerp's empty *Population* and typing "530000" opened the editor with the typed digits, and Enter wrote `530000`. With *Amsterdam* selected, the property grid's *Visited* tick written as unticked (the cell removed, as in the golden); with *Antwerp* selected, *Founded* "1291-01-01" and *Opens* "10:30" typed into the grid and written. Double-click on Amsterdam's *Country* opened the searchable list of the six countries; *Belgium* wrote `o2`. Antwerp's *Tags* list, *Capital*, wrote `o3, o4` and the list stayed open for more, as a list of several does. Antwerp's *Country* list, "Luxembourg" typed and Enter, made the option (`colour: gray`) and gave it to the cell. Typing "a great many" into Amsterdam's *Population* and Enter left the editor open with "A number is expected." under it, and nothing was written; Escape closed it.
- **4/3** *+ New* added an empty row at the foot and wrote its id; *Utrecht* and `361924` were typed into it. The table scrolled sideways to *Province*; F2 on the new row's cell listed the five provinces of `provinces.yaml`, and *Zuid-Holland* wrote `rows: - row: zh` and closed the list (the column's `limit: one`). The relation is one-way: `provinces.yaml` was not written.
- **4/4** *All cities* showed the other view (Sort (1), Filter (1), Group: Country) and wrote `activeView: v1`. The *Name* header's menu, *Sort descending*, added the sort `p1 descending`; the Sort panel's *Descending* beside *Population* turned it to ascending (the `direction` line removed, as in the golden). The Filter panel's *Add a filter…*, *Population*, added a condition (`operator: equals`), its comparison list *is greater than* changed it to `greater-than`, and "500000" in its field wrote `number: 500000`. The Group panel's *Hide empty groups* wrote `hideEmptyGroups: true` and the table showed no groups, since no Dutch city has more than 500,000 people now. The script then folds the *Belgium* group, which is hidden: to reach it, *Hide empty groups* was unticked (the line removed), *Belgium*'s chevron pressed (`collapsed: - key: o2`), and *Hide empty groups* ticked again. The *Visited* header's menu, *Hide*, wrote `visible: false`. Widening *Population* to 200 could not be done (K1). The view tabs' *+* added a view "View" and showed it; its menu's *Rename*, "Recent", Enter wrote nothing and left the field open (K2).
- **4/5** Ctrl+Z after the relation took `p5` out of the new row and the file was the bytes of the step before (same SHA-256); Ctrl+Y put it back (same SHA-256 as after the relation). Ctrl+Z after *+* took the view out and the file was the bytes before it, once a cell had been pressed (K3).
- **4/6** *Map* again (`activeView: v2`), then Antwerp's row menu, *Delete row*: the row was deleted at once, without a question, as the standalone host deletes a row nothing relates to (`KnowledgeEdits.cs`, the question only when rows of other tables relate to it). The body after the sequence, with the two generated ids replaced by the golden's:

| Format | Golden SHA-256 | Desktop SHA-256 (ids replaced) | Differences |
|---|---|---|---|
| YAML | `cd7cd200ecd06fc6…` | `ca6d0772330e4a1b…` | `width: 200` is `width: 175` (K1); `hideEmptyGroups: true` stands before `collapsed:` rather than after it |
| JSON | `db549924deffd8c1…` | `1f877ab6af938882…` | the same two, in the one line of `v1` |
| XML | `ecf679db8565c3f9…` | `4d5faf8f4c93218d…` | `<column property="p3" width="200"/>` is `width="175"` (K1) |

Every other byte matches, line endings (CRLF, as the fixtures) included; `cities.adp` and `provinces.yaml` keep their bytes in all three. The place of `hideEmptyGroups` follows from the walk's order, not from the host: the script folds the group after hiding empty groups, and both hosts then put `collapsed` before `hideEmptyGroups` (the `adp-parity` replay of the script matches the golden), while the walk ticked *Hide empty groups* again after folding, and a key added later is put straight after `groupBy`. The XML form keeps the two in a fixed place, so it shows the width alone.

#### 4/7 Relations between files (YAML)

With `cities.yaml` and `provinces.yaml` open in two tabs: *Zuid-Holland* renamed to "Zuid-Holland (ZH)" in the provinces tab wrote `provinces.yaml` alone, and the cities tab showed Utrecht's province as "Zuid-Holland (ZH)". *Delete row* on *Noord-Holland*, which Amsterdam relates to, asked "1 row in another table relates to this row. Deleting it leaves that relation pointing at nothing." with *Delete* and *Keep* (K5 for where); *Keep* wrote nothing. `text: Overijssel` changed to `text: Overijssel (OV)` in `provinces.yaml` by another program: within three seconds the cities tab showed Enschede's province as "Overijssel (OV)". The property grid shows a relation by the related rows' ids (`nh`) with "A relation is filled in in the table, where its rows can be chosen.", as the standalone host's `KnowledgeRowPropertyProvider` does.

#### 4/8 Change on disk (YAML)

After *Berlin* was renamed "Berlin (B)" in the table, `text: Koeln` was changed to `text: Cologne` on disk by another program: within three seconds the table showed *Cologne* and still *Berlin (B)*, and the file held both. Ctrl+Z then said "cities.yaml was changed by another program, so the edits made to it before that can no longer be undone." in the status bar and wrote nothing.

#### 4/9 Ten thousand rows (release build)

Opening `places.yaml` from the explorer showed its first rows within two seconds of the press; the process's private bytes went from 471,732 KB to 834,776 KB. The vertical scroll bar's thumb was then dragged from the top to the bottom and back five times, 350 pointer moves a pass over 18 seconds each: every capture taken during a drag showed whole rows (no line waiting to be given), the last pass ended on *Place 9999* and *+ New*, and the private bytes were 837,144 KB after the first fifth of the first pass and 838,624 KB at the end of the fifth, flat over the last two passes and a half. The process used about one second of processor time a pass. 200 wheel notches scrolled 242 rows in 6.3 seconds with 0.8 seconds of processor time, at 840,148 KB. `places.yaml` was never written. The headless benchmark at the same commit, release: opened in 747 ms; 552 screens, mean 6.50 ms, 95th percentile 7.09 ms, slowest 11.17 ms a frame; memory held after the warm-up 126 KiB, at the end 49 KiB, peak 131 KiB, at most 44 lines held. No stall was seen in the window, and the memory does not grow with each screen.

### Failures and differences

| # | Formats | What was done | Expected (standalone host at `8b6ccfc`) | What happened |
|---|---|---|---|---|
| K1 | all | Dragged the right border of *Population*'s header to widen it; dragged the header to move it | A header's right border is a separator that resizes the column, sent once on release as `resizeColumn` with the width (never under 60), and dragging the header moves the column (`moveColumn`) (`ColumnHeader.tsx`) | Neither drag does anything: the header has no border to grab and is not dragged. The script's step 24 (`width: 200`) cannot be made, so every format's body differs from its golden there |
| K2 | all | The active view tab's menu, *Rename*, typed "Recent", pressed Enter; pressed elsewhere | Enter or leaving the field renames the view (`renameView`) | Enter and a press elsewhere leave the field open and write nothing; only Escape closes it. A view cannot be renamed |
| K3 | YAML | Escaped the rename field of K2, pressed Ctrl+Z | Ctrl+Z undoes the last edit | Ctrl+Z and Ctrl+Y do nothing, and say nothing, until a cell is pressed: the closed field keeps the keyboard focus, and the shell does not undo while anything but the grid has it |
| K4 | all | Scrolled the table sideways (*Map*, to *Province*) | The first column stays in sight over the cells that scroll under it, its header too (`table-cell-first`, sticky) | The first column is drawn under the scrolled cells: their values and tags are drawn over its tags, and the scrolled headers hide its header |
| K5 | YAML | *Delete row* on a row another table relates to | The question and its *Delete* and *Keep* stand on the table's own line, after the rows (`TableSurface.tsx`, `table-ask` after the rows) | The question's line is drawn just under the header, over the first row |
| K6 | all | Pressed a column's header; pressed it with the right button; looked for the view's properties and groups | Pressing a header opens its menu, with the Knowledge designer's `columnActions`: Rename, Change type, Edit options, Edit relation, Filter, Sort ascending, Sort descending, Group, Hide in view, Wrap text, Freeze, Insert left, Insert right, Duplicate, Delete (`columnActions.ts`, `knowledgeDefinition.ts`); a view's panel ticks each property to show or hide it (`showColumn`) and shows or hides each group (`GroupEditor.tsx`); rows have ticks to select several, with actions on them (`TableSurface.tsx`) | A left press does nothing; the right button's menu offers only Sort ascending, Sort descending, Group by this, Hide and Delete. A hidden column cannot be shown again, a column cannot be renamed, retyped, wrapped, frozen, inserted beside or duplicated, its options and its relation cannot be edited, a group cannot be hidden on its own, and only one row can be selected |
| K7 | all | Opened the *Map* view, whose *Province* says `calculate: count-unique` | A line of calculations under the rows shows the figure and offers the calculations (`CalculationRow.tsx`) | No line of calculations is drawn, and no calculation can be chosen |
| K8 | all | Looked at the property grid's *Visited* of a row without the tick, and at the Group panel's *Hide empty groups* while it is off | A checkbox that is not ticked shows its box | Only the label shows; the box is drawn when it is ticked. The table's own tick cells draw their boxes |
| K9 | all | Turned the mouse wheel over the 10,000 rows | A notch scrolls the browser's step, 100 pixels, about three rows | A notch scrolls about 1.2 rows (40 points): 8,000 notches from top to bottom |

Also seen: the opened 10,000-row table takes about 360 MB of private memory more than before it was opened (4/9), for a 1.7 MB file; not compared with the standalone host. Relation values are drawn underlined, like links, but do nothing when pressed; the standalone Knowledge designer draws them as text, as it passes no `onOpenValue`.

### Re-walk after the fixes

| | |
|---|---|
| Date | 2026-10-21 |
| Platform | As above, a debug build of `adp-shell` (branch `features/030-table-fixes`) at `e2fda5f` for the XML steps and at `e5af4a4` for the rest, and a release build at `e5af4a4` for 4/9; dark theme, window 1600 by 1000 at 125%, the properties pane open |
| Documents | The XML folder as the first walk left it after its whole sequence (`width="175"` the one difference from the golden), and fresh copies of `cities-yaml/` and of the 10,000-row `places.yaml` |
| How | As above: the real window driven with Win32 input and read through its AccessKit tree, every written file hashed (SHA-256) after every step, screenshots with `PrintWindow`. Each fix also has a headless test that fails before it and passes after (`crates/adp-shell/src/table/tests.rs`, `crates/adp-shell/src/theme.rs`); the memory is counted by the instrumented allocator of `benches/table.rs` and in the window as the process's private bytes |
| Standalone behaviour | Read again at `8b6ccfc` for each finding: `ColumnHeader.tsx`, `columnActions.ts`, `OptionsEditor.tsx`, `RelationEditor.tsx`, `CalculationRow.tsx`, `TableSurface.tsx`, `ViewBar.tsx`, `GroupEditor.tsx` and `table.css` |

| # | Fixed | Test | Re-walked in the window |
|---|---|---|---|
| K1 | yes: a strip over each header's right edge is dragged to resize its column, which follows the pointer while nothing is written, and is sent once on release as `resizeColumn`, never under 60; a header dragged moves its column where it is let go (`moveColumn`, `dropIndex`) | `k1_a_columns_border_dragged_resizes_it_once_on_release_and_never_under_sixty`, `k1_a_columns_header_dragged_moves_the_column` | XML, *All cities*: *Population*'s border dragged 31 pixels: the file's hash unchanged while dragging, `<column property="p3" width="200"/>` written on release. YAML, *Map*: *Population*'s header dragged past *Tags*: its `- property: p3` line moved after `p4`'s; Ctrl+Z gave back every byte |
| K2 | yes: the rename field asks for the focus once, so Enter and a press elsewhere keep the name; the key that leaves it is the field's (found on this re-walk: the Enter also opened the active cell's editor) | `k2_a_view_is_renamed_in_its_tab_by_enter_or_a_press_elsewhere` | XML: *+* added *View*, its menu's *Rename*, "Recent" and Enter wrote `name="Recent"`. YAML: *Map* renamed "Places" with Enter: written, and no cell's editor opened; Ctrl+Z gave back every byte |
| K3 | yes: leaving the field gives the keyboard back to the table | `k3_escape_leaves_the_rename_field_and_gives_the_keyboard_back_to_the_table` | XML: *Rename* again, "xyz", Escape: nothing written; Ctrl+Z at once gave back the bytes before the rename, and a second the bytes before *+* |
| K4 | yes: the first column, and the columns a view freezes (*Freeze up to column*), are drawn after the cells that scroll under them, header and line of calculations too | `k4_the_first_column_is_drawn_over_the_cells_that_scroll_under_it` | YAML, *Map*, four notches sideways: *Country* stays at the left over *Name*'s values and *Tags*' tags, its header over theirs |
| K5 | yes: the rows are drawn in a `show_viewport` and the line of calculations, a refusal and a question stand after the rows, kept at the foot while the rows scroll under them (`position: sticky; bottom: 0`) | `k5_the_question_of_a_delete_stands_after_the_rows` | `provinces.yaml`, *Delete row* on *Noord-Holland*: the question with *Delete* and *Keep* on its own line after *+ New* and the line of calculations; *Keep* wrote nothing |
| K6 | yes: a press on a header opens its menu, the Knowledge designer's `columnActions` in `MENU_ORDER` (Rename in the header, Change type, Edit options and Edit relation in their panels, Filter, Sort ascending and descending, Group, Nest where a column offers it, Hide in view, Wrap text, Freeze up to column, Insert left and right named at once, Duplicate, Delete, a two-way relation's Delete asking what becomes of the other side); the view bar's *Properties* panel ticks a property to show or hide it; the *Group* panel hides and shows each group and shows the hidden ones again; each row has a checkbox, the first header one for all, and a bar for the ticked rows sets a property of all of them (`setCells`) or deletes them | `k6_a_columns_menu_offers_the_knowledge_designers_column_actions`, `k6_pressing_a_header_opens_its_menu_and_a_column_is_renamed_in_its_header`, `k6_a_hidden_column_is_shown_again_from_the_views_properties`, `k6_groups_are_hidden_one_by_one_and_shown_again`, `k6_rows_are_ticked_and_acted_on_together` | YAML: a left press on *Population* listed the 13 entries of a number column; *Hide in view* wrote `visible: false`, and *Properties*, *Population* ticked, gave back every byte. *All cities*, *Group*: *Hide* beside *Netherlands* wrote `hiddenGroups: - key: o1`, and *Show 1 hidden group* gave back every byte. *Map*: two rows ticked showed "2 rows selected", *Delete 2 rows* and *Clear selection*; *Edit a property…*, *Visited*, *Check* wrote Antwerp's tick in one step, which one Ctrl+Z gave back |
| K7 | yes: the line of calculations under the rows shows each column's calculation and figure, and a press on a column's place offers *None* and its type's calculations (`setColumnCalculation`) | `k7_the_line_of_calculations_shows_and_offers_each_columns_calculations` | YAML, *Map*: *Province* shows "COUNT UNIQUE VALUES 3"; *Population*'s place offered the 13 calculations of a number, *Sum* wrote `calculate: sum` and showed "SUM 1276643"; Ctrl+Z gave back every byte |
| K8 | yes, in the shell's egui style: a checkbox's box takes VS Code's `checkbox.background` and the border of a secondary button and an input, so every checkbox shows its box | `an_unticked_checkbox_draws_its_box` | Antwerp's *Visited* in the property grid and *Population* in the *Properties* panel, both unticked, show their boxes |
| K9 | yes: a notch scrolls 100 points, the browser's step, everywhere in the shell | `k9_a_notch_of_the_wheel_scrolls_the_browsers_hundred_points` | `places.yaml`: ten notches from the top put *Place 30* at the top, about three rows a notch (the first walk: 1.2) |

Also fixed: a relation's rows are drawn as text, not as links that do nothing (`a_relation_is_drawn_as_text_not_as_a_link`; walked: *Noord-Holland* plain in *Province*); and opening a large table holds less. Counted headless for the 10,000-row table (`benches/table.rs`, the instrumented allocator), opening held 308 MiB, of which the FBL reading kept for the byte-preserving splices held 262 MiB (its element list 127 MiB, its entries 66 MiB, the YAML tree 50 MiB), the reading's model 15 MiB, the session's copy of that model 15 MiB, the Knowledge table 5 MiB a copy, egui's three frames 0.5 MiB. The reading's lists are now shrunk to what they keep once read, an element's id, source and target reads and an entry's xml element and statement are boxed, and the session shares the body's model until an edit is made: opening holds 182 MiB, which the benchmark now bounds at 220. In the window (release build) the private bytes grew from 471,112 KB to 705,476 KB on opening `places.yaml`: 234,364 KB, where the first walk saw 363,044 KB. Nothing read or written changed: the three Knowledge parity tests and every other still match their goldens.

The body after the whole sequence, against the golden, with the two ids the first walk generated replaced by the golden's:

| Format | Golden SHA-256 | Desktop SHA-256 (ids replaced) | Differences |
|---|---|---|---|
| XML | `ecf679db8565c3f9…` | `ecf679db8565c3f9…` | none: the first walk's file with step 24 (`width="200"`) made by dragging, steps 25 to 27 (*+*, *Rename* "Recent", two undos) and step 28 (*Map*) made again |
| YAML, JSON | | | not re-walked: their bodies differed by K1's width and the walk's own order of *Hide empty groups* (above), which a re-walk would repeat |

Seen on the re-walk and not fixed: the *Group* panel lists its groups without the handle that drags one into another place (`moveGroup`) and without *Order of groups* (`sortGroups`); a row has no handle that drags it (`moveRows`); *Insert left* and *Insert right* offer the kinds in a menu and name the new property in its header, where the standalone host asks name and kind in one panel (`NewColumnPanel.tsx`); and the view bar names its four panels as buttons rather than behind one control with the filters and sorts as items beside it.

| Step | YAML | JSON | XML |
|---|---|---|---|
| 4/4 | pass | not re-walked | pass |
| 4/5 | pass | not re-walked | pass |
| 4/6 | as before: differs only by the walk's order (not re-walked) | as YAML | pass, byte for byte |
| 4/7 | pass | not walked | not walked |
| 4/9 | pass (`places.yaml`, release) | | |

### Screenshots

- [The YAML example in its *Map* view](walks/story4-yaml-map-view.png)
- [The list of a selection cell](walks/story4-option-list.png)
- [The list of a relation cell, the rows of `provinces.yaml`](walks/story4-relation-list.png)
- [A refused number, the editor left open](walks/story4-refused-number.png)
- [Scrolled sideways, the first column drawn under the others (K4)](walks/story4-first-column-overdrawn.png)
- [The view rename field that cannot be committed (K2)](walks/story4-view-rename-stuck.png)
- [The delete question drawn over the first row (K5)](walks/story4-delete-question-over-header.png)
- [A change on disk shown with the person's edit, and undo refused](walks/story4-disk-change-barrier.png)
- [Related rows renamed in the other file, by hand and on disk](walks/story4-related-file-changed.png)
- [10,000 rows at the bottom](walks/story4-10000-rows-bottom.png)
- [10,000 rows during a drag of the scroll bar](walks/story4-10000-rows-mid-drag.png)
- [Re-walk: *Population*'s border dragged, nothing written yet (K1)](walks/story4-rewalk-resize-drag.png)
- [Re-walk: scrolled sideways, the first column over the others; a relation as text; the line of calculations (K4, K7)](walks/story4-rewalk-first-column-over.png)
- [Re-walk: an unticked checkbox in the property grid (K8)](walks/story4-rewalk-unticked-checkbox.png)
- [Re-walk: a header's menu, opened by a press (K6)](walks/story4-rewalk-column-menu.png)
- [Re-walk: the *Properties* panel with a hidden column (K6, K8)](walks/story4-rewalk-properties-panel.png)
- [Re-walk: a group hidden from the *Group* panel (K6)](walks/story4-rewalk-group-hidden.png)
- [Re-walk: two rows ticked and the bar for them (K6)](walks/story4-rewalk-ticked-rows.png)
- [Re-walk: *Sum* chosen for *Population* (K7)](walks/story4-rewalk-calculation-sum.png)
- [Re-walk: the delete question after the rows (K5)](walks/story4-rewalk-delete-question.png)
- [Re-walk: ten wheel notches over 10,000 rows (K9)](walks/story4-rewalk-ten-notches.png)

## The hierarchical requirement designer, Story 7: the page as the maintainer shaped it (etalii.adp spec 036, task T054)

| | |
|---|---|
| Date | 2026-10-25 |
| Platform | Ubuntu 24.04.5 in a cloud container, X virtual frame buffer at 1580 by 900, no compositor: a release build of `adp-shell` (`target/release/adp-desktop`) on branch `features/036-walks`, beside the standalone host at [`a7fdf03`](https://github.com/etalii-adp/etalii.adp.ide.standalone/tree/a7fdf03) (its backend from its own build, its client from `vite`, in Chromium 1243 driven by Playwright 1.63 at the same 1580 by 900). Dark theme; light theme with a walk-only build that sets the desktop host's theme preference to light, because winit reports no system theme on X11 and the desktop host then stays dark |
| Documents | One copy each of the web shop example (`web-shop.md` with its registration `web-shop.adp`), byte for byte the same in both hosts' folders, opened as a project |
| How | Both windows at the same size, screenshots of each at every point, compared by eye and, for sizes, by the pixel rows and columns of the screenshots; the desktop host's bounds read from its accessibility tree through AT-SPI, the standalone host's from the DOM (`boundingBox`); every written file hashed (SHA-256) and the two hosts' files compared byte for byte after the same gesture |
| Standalone behaviour | The standalone host at `a7fdf03` itself, and its source there (`HierarchySurfaceBox.tsx`, `iconsLayout.ts`, `hierarchySurface.css`, `columns/`) |

### Results

| Point | Scenario | Result |
|---|---|---|
| 7/1 | A column's edge dragged: every box of that depth follows, one step of undo, the width written in `widths` | pass: the same drag wrote the same `web-shop.adp` bytes in both hosts, and Ctrl+Z gave back the bytes before it |
| 7/2 | Double-click or Home resets, the arrows step by 16, a refused gesture puts the column back | pass |
| 7/3 | A width outside 120 to 2000, or not a number, is ignored | pass: `widths` edited to `90`, `5000` and `wide` by hand left the column at the page's width in both; a valid `300` applied in both |
| 7/4 | One icon list, assessment first; a row at the title's end when everything fits, otherwise a column at the right | pass |
| 7/5 | A wrapped title: handle, collapse and number as words of the first line, aligned to the top, later lines 12 from the box's edge | pass: the second line's first ink at 13 pixels from the box's edge, the standalone host's at 13 to 14 |
| 7/6 | Padding 6, labels and description 12 from the left, 12 below the description | pass after fix W2 |
| 7/7 | The same height unselected, selected and open for editing | pass in the desktop host: REQ-0003 is 104 pixels high in all three states in both hosts. The standalone host grows a box whose tags, category or state are empty when it is selected (REQ-0014, which has none, 32 to 52; REQ-0008 74 to 92), which the scenario forbids; the desktop host keeps the height (W5), and the difference is recorded below |
| 7/8 | Hover controls hidden until the pointer is over the box; add item and add requirement are icons; "n more levels" at every level | pass |
| 7/9 | The trail follows the selection and names items only | pass |
| 7/10 | A new item or requirement is revealed and selected | pass |
| 7/11 | The title field is one line high and starts after the number | pass |

### Fixes made on this walk

Each has a headless test in `crates/adp-shell/src/hierarchy/tests.rs` (or `theme.rs`) that was seen to fail against the defect before it passed.

| # | What was seen | Fixed | Test |
|---|---|---|---|
| W1 | The sort control's icon was written into the menu's text, in a font without the glyph, so it showed as a box | The icon stands before the menu as its own label in the icons' font, as the filter's does before its field | `the_sort_menu_shows_its_choice_alone_its_icon_before_it_in_the_icon_font` |
| W2 | Less than 12 pixels stood below a description | The space below a description is the description's top margin, its height and its bottom margin | `a_description_stands_4_below_the_tags_and_12_above_the_boxs_bottom_edge` |
| W3 | The filter field kept its width when the bar's controls changed, and a stored filter's controls pushed the count out of the window | The field takes what the bar leaves, measured against the bar's right edge before anything is drawn | `the_field_takes_what_the_bar_leaves`, `the_field_gives_way_when_a_stored_filter_brings_its_controls_and_nothing_leaves_the_window` |
| W4 | In the light theme, warnings were drawn in plain yellow, hard to read on the white page | Warnings take the theme's warning colour (`editorWarning.foreground` and its fallbacks), checked for a contrast of at least 3 in both themes | `a_warning_reads_on_the_editors_background_in_both_themes` |
| W5 | Tags, category and state without a value show their picker only on the selected box, and selecting a box made it higher | An unselected box keeps the picker's place without drawing it, so selecting it keeps its height | `the_way_to_give_a_value_keeps_its_place_on_a_box_that_is_not_selected_so_selecting_it_keeps_its_height` |

### Differences recorded, not fixed

- The standalone host grows a box on selection (7/7); the desktop host follows the scenario. The keeping of the picker's place leaves a gap before a state on a box that has no tags or category, where the standalone host draws nothing until it is selected.
- Fonts: the desktop host draws egui's Ubuntu-derived font where the standalone host draws the browser's, so text runs a few pixels wider or narrower, and bold titles are not bold.
- Colours are VS Code's (desktop FR-044, spec 036 FR-008): in the light theme the item column is (250, 250, 253) where the standalone host's is (240, 240, 240).
- The trail's links are not underlined, and the pickers are drawn as pills.

### Screenshots

- [Dark, desktop host](walks/hierarchy-story7-desktop-dark.png) and [standalone host](walks/hierarchy-story7-standalone-dark.png)
- [Light, desktop host](walks/hierarchy-story7-desktop-light.png) and [standalone host](walks/hierarchy-story7-standalone-light.png)
- [A requirement added, revealed and selected, its title open: desktop host](walks/hierarchy-story7-desktop-added.png) and [standalone host](walks/hierarchy-story7-standalone-added.png)
- [After the fixes, dark, desktop host](walks/hierarchy-after-dark.png)

## The hierarchical requirement designer, Stories 1 to 6 (etalii.adp spec 036, task T059)

| | |
|---|---|
| Date | 2026-10-25 |
| Platform | As for Story 7 above, both themes |
| Documents | The same two folders, each with `web-shop.md` and `web-shop.adp`, and four documents made for the walk, the same bytes in both: `agent.md` (a requirement document an agent writes to), `plain.md` (Markdown without an envelope), `later.md` (an envelope of DED 0.3, later than either host reads) and `fresh.md` (a new document from the template) |
| How | As for Story 7. The changes "from another program" were written with Python into both folders at once. The screen reader pass was made with the AT-SPI tree, read with `pyatspi` on the session bus with the screen reader flag on, as Orca would read it; no screen reader was run and nothing was heard, so this is a reading of what one would be told, not a walk with one |
| Standalone behaviour | The standalone host at `a7fdf03` itself, beside it |

### Results

| Story | What was done | Result |
|---|---|---|
| 1 | The add dialog's catalogue; `web-shop.md`, `agent.md` without a registration, `plain.md` and `later.md` opened from the explorer; levels, the mark of hidden levels, focus, the trail, collapse | pass: the same etalii designers listed in both; an envelope without a registration opens in the designer, plain Markdown in the Markdown editor, DED 0.3 read-only; the same entries shown in both. The read-only reason now says the standalone host's sentence (W6) |
| 1, screen reader | The AT-SPI tree of the open web shop page | pass after fix W7: the page is a tree named by the document, its boxes tree items with their numbers and titles, every picker, collapse control and grip a button named for its box, the trail and the filter bar landmarks, the filter an entry, sort and stored filters combo boxes, the count a status. [The tree as read](walks/hierarchy-atspi-web-shop.txt) |
| 2 | Another program moved and retitled entries in `agent.md` while it was open with one entry collapsed; then Ctrl+Z | pass: the move and the new title shown at once, the collapsed entry still collapsed; Ctrl+Z said the same sentence as the standalone host and wrote nothing |
| 3 | The conformance run | pass: 59 tests, none set aside (`cargo test -p adp-conformance`) |
| 4 | `fresh.md` made from the template and given a title; items and requirements added; one dragged to another level | pass: the same bytes as the standalone host's after the title, with no registration written; the drop preview the same, the drop writing the same bytes. The desktop host prefills the new document's name with `hierarchical-requirement` |
| 5 | Notes added, listed from the icon, resolved | pass: the same list, the same bytes after adding and after resolving, the icon of several notes in both |
| 6 | `@pro` completed; the `@proposed` filter; the filter stored; a state picked | pass after fixes W8 and W9: completion offers `@proposed`, the filter leaves the same entries, storing it and picking a state write the same `.adp` and `.md` bytes in both; the three looks of the labels (tags, category, state) are distinct |

### Fixes made on this walk

| # | What was seen | Fixed | Test |
|---|---|---|---|
| W6 | The read-only reason of a later DED version was the desktop host's own sentence | It is the standalone host's: "This document is written in DED 0.3, and this application reads DED 0.2 and earlier. It is left exactly as it is and cannot be changed here." | `modules/designers/hierarchical-requirement/tests/session.rs` |
| W7 | The screen reader was told the filter's and the sort's icons as labels named by their private-use glyphs (`\U000f0236`, `\U000f04ba`), and the trail's chevrons, the read-only lock, a finding's mark and a note's kind as text runs of their glyphs | The icons are painted, not laid out as labels: decorative ones are hidden from the accessibility tree, and those that say something (a finding's severity, a note's kind) are named | `a_screen_reader_reads_no_icon_as_its_glyph_in_the_bars_the_trail_or_the_page`, and the same check in the Store and notes list tests |
| W8 | The Store button's icon was written into its text, in a font without the glyph | The icon is drawn in the icons' font and the word in the page's | `the_store_control_draws_its_icon_in_the_icon_font_and_its_word_in_the_pages` |
| W9 | The list of completions opened above the field, over the trail | It opens under the field, as the standalone host's | `the_list_of_completions_stands_under_the_field` |
| W10 | A dragged level was faded alone, while what it carries was drawn as before | Everything the dragged item carries is faded with it | `a_dragged_level_is_faded_with_everything_it_carries` |

### Differences recorded, not fixed

- The notes list's *Add* is a plain button, where the standalone host's is filled green; the count of several notes is drawn small and low, like a subscript.
- The list of completions is as wide as the field; the standalone host's is as wide as its entries.
- On X11 the desktop host cannot follow a light system theme (above).
- Seen in the standalone host, not in the desktop host: after another program wrote `web-shop.md`, selecting REQ-0014 offered no *New requirement below* in the ribbon, Ctrl+Enter did nothing, and "This item is no longer available." was shown, also after the project was opened again. The desktop host added below it as before.

### Screenshots

- [Story 1, `agent.md` opened by its envelope: desktop host](walks/hierarchy-s1-d-agent.png) and [standalone host](walks/hierarchy-s1-s-agent.png)
- [Story 1, `later.md` read-only: desktop host](walks/hierarchy-s1-d-later.png) and [standalone host](walks/hierarchy-s1-s-later.png)
- [Story 2, the agent's change shown: desktop host](walks/hierarchy-s2-d-agent.png) and [standalone host](walks/hierarchy-s2-s-agent.png); [undo refused in the desktop host](walks/hierarchy-s2-d-undo.png)
- [Story 4, the drop preview: desktop host](walks/hierarchy-s4-d-drag.png) and [standalone host](walks/hierarchy-s4-s-drag.png)
- [Story 5, the notes list: desktop host](walks/hierarchy-s5-d-notes.png) and [standalone host](walks/hierarchy-s5-s-notes.png)
- [Story 6, completion: desktop host](walks/hierarchy-s6-d-complete.png) and [standalone host](walks/hierarchy-s6-s-complete.png); [after W9, the list under the field](walks/hierarchy-after-complete.png)
- [Story 6, a stored filter: desktop host](walks/hierarchy-s6-d-stored.png) and [standalone host](walks/hierarchy-s6-s-stored.png)
