<!-- ded: 0.2 designer: etalii/post-incident-review -->
<!-- last: E-0008 F-0003 -->
# Checkout latency, 3 November 2026

An invented review, drafted by an agent from the alert, the change log and the incident channel, and not yet judged by a person: eight timeline entries, three proposed factors and no actions.

## How to read this file

This file is one post-incident review, kept by people and agents together. It records the conditions that let an incident happen and what is changed so that it does not happen again; no line names a person's fault. Leave its first two lines as they are.

- Under `Summary`, `- Time zone: <IANA name>` names the zone the review is read in, and the other item is the summary. Under `Impact`, the items are `- Affected:`, `- Effect:` and `- Duration:`.
- Under `Timeline` each item is one entry: its number, its time (RFC 3339 with its offset, such as `2026-11-03T10:12+01:00`), a milestone before a colon when it is one (`trigger`, `start`, `detection`, `engagement`, `mitigation start`, `mitigated` or `resolved`), its text, its author (`by:agent` or `by:person`) and its evidence as links whose text has no spaces.
- Under `Contributing factors` each item is one factor: its number, `category:<name>`, `state:proposed`, `state:accepted` or `state:rejected`, its author and tags (`#name`), its text, then the entries it rests on (`[E-0003]`) and its evidence. An agent proposes; a person accepts or rejects. A rejected factor stays, with the reason indented under it.
- Under `Actions` each item is one action: `[ ]` open or `[x]` done, its number, its text, `owner:<who carries it>`, `due:<YYYY-MM-DD>` and `addresses:[F-0001]`, once for each factor it addresses.
- Under `Categories` each item is one category of factor; others can be added.
- A number is never changed and never used again. Add an entry, factor or action with `E-NEW`, `F-NEW` or `A-NEW` in place of a number: the designer numbers it. Lines indented under an item are its detail.

```markdown
- E-NEW 2026-11-03T10:12+01:00 detection: Latency alert fired. by:agent [alert](https://example.org/alerts/1)
- F-NEW category:detection state:proposed by:agent The alert threshold is above the customer-visible delay. [E-0003]
- [ ] A-NEW Lower the alert threshold to 1 s. owner:@platform due:2026-12-01 addresses:[F-0002]
```

## Summary

- Time zone: Europe/Berlin
- Checkout requests slowed to over 8 seconds after a configuration change lowered the payment gateway's connection pool. Rolling the change back ended the impact.

## Impact

- Affected: customers checking out on the web shop
- Effect: 38 % of checkouts abandoned
- Duration: 57 minutes, from start to mitigated

## Timeline

- E-0001 2026-11-03T09:58+01:00 trigger: Connection pool size lowered in a configuration change. by:agent [change](https://example.org/changes/481)
- E-0002 2026-11-03T10:05+01:00 start: p95 latency of checkout passes 2 s. by:agent [dashboard](https://example.org/dashboards/checkout)
- E-0003 2026-11-03T10:12+01:00 detection: Latency alert fired. by:agent [alert](https://example.org/alerts/9312)
- E-0004 2026-11-03T10:20+01:00 engagement: On-call engineer acknowledged the alert. by:agent
- E-0005 2026-11-03T10:41+01:00 mitigation start: Configuration change rolled back. by:agent [change](https://example.org/changes/482)
- E-0006 2026-11-03T11:02+01:00 mitigated: Latency back under 1 s. by:agent
- E-0007 2026-11-03T11:30+01:00 resolved: Incident closed. by:agent
- E-0008 2026-11-03T10:33+01:00 Connection errors to the payment gateway found in the logs. by:agent [log](https://example.org/logs/q/77)
  Found after the timeline was first drafted, so written last; the designer shows it in time order.
- E-NEW 2026-11-03T10:15+01:00 The status page was updated to say checkout is slow. by:agent [status](https://example.org/status/2211)
- E-NEW 2026-11-03T10:50+01:00 Error rate of checkout back to normal. by:agent
- E-NEW 2026-11-03T11:10+01:00 Customers told by e-mail that checkout works again. by:agent

## Contributing factors

- F-0001 category:change state:proposed by:agent Configuration changes skip the canary stage. [E-0001]
- F-0002 category:detection state:proposed by:agent The alert threshold is above the customer-visible delay. [E-0002] [E-0003]
- F-0003 category:capacity state:proposed by:agent The configuration schema sets no lower bound for the connection pool. [E-0001] [E-0008]

## Actions

## Categories

- change
- detection
- capacity
- dependency
- people
