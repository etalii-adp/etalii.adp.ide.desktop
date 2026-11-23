# Post-incident review designer

The companion of [post-incident-review.des](post-incident-review.des): what the designer type is for, the file in full, how the intervals are computed, and what DESL and FBL cannot say. Origin `etalii/post-incident-review`, kind Designer.

## Blameless by intent

A post-incident review records the **conditions** that let an incident happen or made it worse, and what is changed so that it does not happen again. It never records whose fault it was. The definition keeps to that by its shape:

- No element type has an attribute for blame, fault, cause by a person or responsibility for the incident (FR-009). A factor is a condition: "configuration changes skip the canary stage", not "the engineer skipped the canary".
- An action's `owner` names who **carries the action**, such as a team, and nothing about who caused anything.
- A factor that names a person's act is rejected and kept, with the reason under it, so the review shows that it was considered and why it was set aside (`certificate-expiry.md`, F-0005).

This is why the review is worth keeping by people and agents together: an agent can assemble the timeline and propose factors from logs, alerts and the incident channel, and the people who were there judge them without having to defend themselves.

## What it is for

One incident's review as a page of six parts: a summary with the time to detect, engage, mitigate and resolve; the impact; the timeline in time order on a time strip with its milestones marked; the contributing factors grouped by category, each with its state; the actions with what is missing; and the category list. A factor names the entries it rests on, and an action the factors it addresses, as values: selecting one highlights the others in their parts (DESL section 7.8.2). Nothing is drawn connected, which is why this is a designer.

## The file

A review is one Markdown file that reads as an ordinary review without any tool (FR-006). The designer claims a `.md` file only when line 1 is the envelope naming this designer type, so other Markdown files are left alone.

```markdown
<!-- ded: 0.2 designer: etalii/post-incident-review -->
<!-- last: E-0008 F-0003 A-0001 -->
# Checkout latency, 3 November 2026

## How to read this file

(the legend for agents, as the template writes it)

## Summary

- Time zone: Europe/Berlin
- Checkout requests slowed to over 8 seconds after a configuration change.

## Impact

- Affected: customers checking out on the web shop
- Effect: 38 % of checkouts abandoned
- Duration: 57 minutes, from start to mitigated

## Timeline

- E-0001 2026-11-03T09:58+01:00 trigger: Connection pool size lowered in a configuration change. by:agent [change](https://example.org/changes/481)
- E-0003 2026-11-03T10:12+01:00 detection: Latency alert fired. by:agent

## Contributing factors

- F-0001 category:change state:accepted by:agent Configuration changes skip the canary stage. [E-0001]
- F-0003 category:people state:rejected by:agent Engineer unfamiliar with the gateway.
  Rejected: the review records conditions, not people.

## Actions

- [ ] A-0001 Run configuration changes through the canary stage. owner:@platform due:2026-12-01 addresses:[F-0001]

## Categories

- change
- detection
```

### Lines 1 and 2

Line 1 is DED's envelope and the binding's required header. Line 2 is the mark line: the last number each counter gave, entries first, then factors, then actions, each left out until its counter has given one (`<!-- last: E-0008 F-0003 -->` before the first action). Both are written by the designer. **An agent leaves lines 1 and 2 as they are.**

### Numbers

Entries are `E-0001` onwards, factors `F-0001` onwards and actions `A-0001` onwards, each counted on its own (DESL section 5.3.5). A number is never changed and never used again, not even after its item is deleted. An agent that adds an item writes `E-NEW`, `F-NEW` or `A-NEW` in place of a number; the designer numbers those with its next edit, in the order of the file, and updates line 2 in the same edit. Until then a host names such an item by its line, `line/44`, and nothing can refer to it: an agent that adds an entry and a factor resting on it in one go writes the factor without `[E-…]`, and adds the reference once the designer has numbered the entry.

### The sections

The six sections are found by their headings, at any level and in any case: the first heading whose text is `Summary`, `Impact`, `Timeline`, `Contributing factors`, `Actions` or `Categories`, up to the next heading with as many `#` or fewer. The title, the legend under `How to read this file`, any other section, paragraphs and fenced blocks are kept byte for byte and are not part of the model. The title is therefore not an attribute: a host shows the file's name.

| Section | Line form | Model |
|---|---|---|
| Summary | `- Time zone: <IANA name>` | the document's `timeZone` |
| Summary | `- <any other text>` | the document's `summary`; of several, the first |
| Impact | `- Affected: …`, `- Effect: …`, `- Duration: …` | the document's `affected`, `effect`, `duration` |
| Timeline | `- E-0001 <time> <milestone>: <text> by:<author> [<link text>](<url>)` | an `Entry` |
| Contributing factors | `- F-0001 category:<name> state:<state> by:<author> #<tag> <text> [E-0001] [<link text>](<url>)` | a `Factor` |
| Actions | `- [ ] A-0001 <text> owner:<who> due:<YYYY-MM-DD> addresses:[F-0001]` | an `Action`, `[x]` when done |
| Categories | `- <name>` | a `Category` |

### A timeline entry

- **The time** is RFC 3339 with its offset or `Z`, the seconds optional: `2026-11-03T10:12+01:00`, `2026-10-14T00:47Z`. It is kept as written (DESL `timezone: "preserve"`), and the designer shows it in the review's time zone. An entry without a time is read, listed after the time strip and reported.
- **The milestone** stands before a colon when the entry is one: `trigger`, `start`, `detection`, `engagement`, `mitigation start`, `mitigated` or `resolved`, in any case. A line without one is an ordinary entry. Only the first entry of each milestone counts in the intervals; a second is reported.
- **The text** is one line.
- **The author** is `by:agent` or `by:person`, at the end. A missing author is reported.
- **Evidence** is Markdown links at the end. The link text has no spaces: `[log](https://…)`, not `[the log](https://…)`, which would be read as part of the text.
- **Lines indented** under the entry are its detail. `[E-0003]` in a detail names an entry.

The entries may stand in any order in the file; the designer shows them in time order and never reorders the file. An agent appends a late finding at the end of the timeline.

### A contributing factor

After the number come its leading words in any order: `category:<name>`, `state:proposed`, `state:accepted` or `state:rejected`, `by:agent` or `by:person`, and tags `#name`. Then the text, a condition stated as a condition. Then its trailing words: the entries it rests on, `[E-0003]`, and evidence links. Of two category, state or author words, the first is read and the other stays as written. A factor without a state is proposed.

**An agent proposes; a person accepts or rejects.** A rejected factor stays in the file with its reason indented under it.

### An action

A task list item: `[ ]` open or `[x]` done, the number, the text, then in any order `owner:<who>`, `due:<YYYY-MM-DD>` and `addresses:[F-0001]`, written once for each factor it addresses (`addresses:[F-0001] addresses:[F-0002]`). An action without an owner or a due date is marked and reported.

### Categories

The category list is the user's. The template starts it with `change`, `detection`, `capacity`, `dependency` and `people` (FR-008); categories can be added, renamed and removed by editing the list. A factor whose category is not in the list is kept and reported. Factors are grouped by category, ignoring case.

### What cannot be read

An item under Impact, Timeline, Contributing factors or Actions in none of these forms is kept as written, with everything nested in it, and reported as an unreadable entry (FR-007). An item under Summary is always readable, and an item under Categories is always a category.

## The JSON form

The validator reads a review into the JSON form [post-incident-review.schema.json](post-incident-review.schema.json) describes: `ded` and `designer` from line 1, `lastEntry`, `lastFactor` and `lastAction` from line 2, `timeZone`, `summary`, `affected`, `effect` and `duration`, then `entries`, `factors` and `actions` in the order of the file, and `categories` as names. A milestone is given by its key, so `mitigation start` is `mitigationStart`; a done action has `done: true`.

## How the intervals are computed

The four figures beside the Summary are DESL expressions over the timeline (FR-003, research R4). `milestones(k)` is the entries of milestone `k` that have a time, in the order of the file; `between(a, b)` is the time of the first entry of `b` minus the time of the first entry of `a`.

| Figure | Expression | From | To |
|---|---|---|---|
| Time to detect | `between('start', 'detection')` | start | detection |
| Time to engage | `between('detection', 'engagement')` | detection | engagement |
| Time to mitigate | `between('detection', 'mitigated')` | detection | mitigated |
| Time to resolve | `between('start', 'resolved')` | start | resolved |

When either milestone is missing, the figure shows "No … entry yet" and the missing milestone is reported. Times with different offsets subtract correctly, since each is an instant. The `Duration:` line under Impact is the author's own words and is not computed.

### Worked intervals

Both examples are invented and say so.

**[checkout-latency.md](examples/checkout-latency.md)**, drafted by an agent: eight entries, three proposed factors, no actions. All times are `+01:00`. E-0008 (10:33) is written last, after E-0007 (11:30).

| Figure | From | To | Interval |
|---|---|---|---|
| Time to detect | E-0002 start, 10:05 | E-0003 detection, 10:12 | 7 min |
| Time to engage | E-0003 detection, 10:12 | E-0004 engagement, 10:20 | 8 min |
| Time to mitigate | E-0003 detection, 10:12 | E-0006 mitigated, 11:02 | 50 min |
| Time to resolve | E-0002 start, 10:05 | E-0007 resolved, 11:30 | 1 h 25 min |

The Impact's "57 minutes, from start to mitigated" agrees: 10:05 to 11:02.

**[certificate-expiry.md](examples/certificate-expiry.md)**, complete: nine entries, five factors (four accepted, one rejected and kept), four actions, one done. F-0004 is accepted and addressed by no action, so it carries the mark *Unaddressed*. All times are UTC (`Z`). E-0009 (00:12) is written last.

| Figure | From | To | Interval |
|---|---|---|---|
| Time to detect | E-0002 start, 00:00 | E-0003 detection, 00:47 | 47 min |
| Time to engage | E-0003 detection, 00:47 | E-0004 engagement, 00:55 | 8 min |
| Time to mitigate | E-0003 detection, 00:47 | E-0007 mitigated, 01:38 | 51 min |
| Time to resolve | E-0002 start, 00:00 | E-0008 resolved, 03:00 | 3 h |

## Findings

| Code | Severity | When |
|---|---|---|
| `pir.no-time` | warning | An entry has no time. |
| `pir.missing-milestone` | info | A milestone an interval needs is absent; one finding per missing milestone, naming the intervals it stops. |
| `pir.second-milestone` | warning | A second entry of one milestone; the intervals use the first. |
| `pir.no-author` | info | An entry or a factor has no `by:`. |
| `pir.unknown-category` | info | A factor's category is not in the list; it is kept. |
| `pir.action-incomplete` | warning | An action has no owner or no due date. |
| `pir.unaddressed` | info | An accepted factor no action addresses. |
| `std.mentions` | warning | `[E-…]`, `addresses:[F-…]` or a mention in a detail names nothing. The data model's `pir.unknown-entry` and `pir.unknown-factor`, reported by DESL's built-in. |
| `std.unreadableEntry` | warning | An item in no known form, kept as written. The data model's `pir.unread`. |
| `std.duplicateId` | warning | A number given twice; the first holder keeps it. |

## The page

The parts are those of the specification's `sections` surface, in order: Summary (a record, with the four figures), Impact (a record), Timeline (a list ordered by time, with the time strip above it emphasising and labelling the milestones), Contributing factors (a list grouped by category, with the marks *Unaddressed* and *Rejected*), Actions (a list, with the mark *No owner or due date*) and Categories. A row without a time is listed after the strip. A rejected factor is shown dimmed but in full. The looks of a part, a badge and the strip are the host's, in its own theme.

## Gaps

- **The title** is a `#` heading, which the bullets family does not bind (FBL section 4.8), so it is kept as text and is not in the model. A host shows the file name.
- **A many attribute in an emit.** FBL 6.2 does not say how the placeholder of a `many` attribute is written. The action's emit writes `addresses:[{addresses}]` for the one factor the designer's add gives; a second factor is added by a set, which writes a word for each item (FBL 6.3).
- **The tool description** (`post-incident-review.atd` with `post-incident-review.markdown.atd`) states the page through ATD's `sections` surface (ATD §9.4), added for it by the ADP tool language's own work (Peter's choice of 2026-11-23); its trace is `post-incident-review.trace.json`.

## What stays with the host

- Showing times in the review's time zone, and the axis and labels of the time strip.
- Evaluating the constraints and drawing their findings beside their rows.
- The looks of parts, marks and figures.

## Notes

- The validator reads both examples and the fixtures under `specifications/fbl/fixtures/post-incident-review-read/` and `post-incident-review-edits/`.
- **An agent given only this companion (T030, 2026-11-23).** A model given this file and a new review from the template, and nothing else, added two timeline entries (start and detection, `E-NEW`, RFC 3339 with `+01:00`, `by:agent`, one evidence link) and one proposed factor, set the time zone and the summary, and left lines 1 and 2 and every other line as they were. The validator's reading of the result reports nothing; a host would report the missing milestones engagement, mitigated and resolved (`pir.missing-milestone`, info), as it should for a review in progress. The one thing it found unclear was whether a factor may cite an entry still written `E-NEW`; the section *Numbers* now says it may not. SC-003 (four of five attempts with eight entries and three factors) is a host's measure and was not run here.
