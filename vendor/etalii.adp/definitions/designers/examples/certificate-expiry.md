<!-- ded: 0.2 designer: etalii/post-incident-review -->
<!-- last: E-0009 F-0005 A-0004 -->
# Expired API certificate, 14 October 2026

An invented review, complete: its factors judged, one rejected and kept, four actions agreed, and one accepted factor that no action addresses yet.

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

- Time zone: UTC
- The TLS certificate of the public API expired at midnight, and the mobile app could not reach the API until a renewed certificate was deployed.

## Impact

- Affected: users of the mobile app
- Effect: no sign-in and no data in the app
- Duration: 1 hour 38 minutes, from start to mitigated

## Timeline

- E-0001 2026-10-14T00:00Z trigger: The TLS certificate of api.example.org expired. by:agent [certificate](https://example.org/certificates/api)
- E-0002 2026-10-14T00:00Z start: Requests from the mobile app fail the TLS handshake. by:agent [dashboard](https://example.org/dashboards/api)
- E-0003 2026-10-14T00:47Z detection: Customer support reports that the app cannot sign in. by:person
- E-0004 2026-10-14T00:55Z engagement: On-call engineer joins the incident channel. by:person
- E-0005 2026-10-14T01:20Z The expired certificate is found to be the cause. by:person [check](https://example.org/checks/tls/1)
- E-0006 2026-10-14T01:31Z mitigation start: Renewed certificate deployed to the load balancers. by:person
- E-0007 2026-10-14T01:38Z mitigated: Handshakes succeed again. by:agent [dashboard](https://example.org/dashboards/api)
- E-0008 2026-10-14T03:00Z resolved: Incident closed after a last check. by:person
- E-0009 2026-10-14T00:12Z The synthetic check of the API reports errors, and nobody is alerted. by:agent [check](https://example.org/checks/synthetic/api)

## Contributing factors

- F-0001 category:change state:accepted by:agent Certificate renewal is a manual step that no schedule triggers. [E-0001]
- F-0002 category:detection state:accepted by:agent No alert watches the expiry dates of certificates. [E-0001] [E-0003]
- F-0003 category:detection state:accepted by:person The synthetic check reports errors without alerting anyone. [E-0009]
- F-0004 category:people state:accepted by:person Renewal reminders go to a shared mailbox that nobody has read since the team changed. [E-0001]
  No action is agreed yet: the mailbox's owner is being found.
- F-0005 category:people state:rejected by:agent The on-call engineer joined late. [E-0004]
  Rejected: the review records conditions, not people. Engagement took 8 minutes, within the target of 15.

## Actions

- [x] A-0001 Move certificate renewal to the automated issuer. owner:@platform due:2026-10-31 addresses:[F-0001]
- [ ] A-0002 Alert 30 days and 7 days before a certificate expires. owner:@observability due:2026-11-15 addresses:[F-0002]
- [ ] A-0003 Page the on-call engineer when the synthetic API check fails. owner:@observability due:2026-11-15 addresses:[F-0003]
- [ ] A-0004 List the other checks that report without alerting. owner:@observability due:2026-11-30 addresses:[F-0003]

## Categories

- change
- detection
- capacity
- dependency
- people
