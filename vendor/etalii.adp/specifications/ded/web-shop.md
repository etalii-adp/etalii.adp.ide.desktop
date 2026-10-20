<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0003 REQ-0004 -->
# Web shop

## How to read this file

This file holds the requirements of one subject. Leave its first two lines as they are.

- The bullets under `Requirements` are items (`ITEM-`) and, inside an item, requirements (`REQ-`) and further items.
- A number is never changed and never used again. Add an entry as `ITEM-NEW` or `REQ-NEW`: it is numbered later.
- Lines indented under a bullet describe it.
- Tags (`#name`), one category (`category:name`) and one state (`state:name`) stand at the end of a bullet's first line.
- Refer to an entry by its number in square brackets, as in `[REQ-0001]`.
- The bullets under `Assessments` are questions, remarks, warnings and ideas about one entry. Write `[x]` to resolve one.

```markdown
- ITEM-NEW Checkout #core
  What a customer does between the basket and the receipt.
  - REQ-NEW An order is confirmed by e-mail. #mail category:functional state:proposed
    Sent after the payment of [REQ-0001] succeeds.

- [ ] Question REQ-0001: Which payment methods count?
```

## Requirements

- ITEM-0001 Backend #core
  Everything behind the public interface.
  - REQ-0001 A session ends after 30 minutes without activity. #security category:functional state:accepted
    Activity is any request the user makes. See [REQ-0002] for sign-in.
  - REQ-0002 Users sign in with an e-mail address and a password. #security
  - ITEM-0002 Sessions
    - REQ-NEW Tokens are refreshed without the user noticing.

## Assessments

- [ ] Question REQ-0001: Is 30 minutes right for the mobile app?
- [x] Warning ITEM-0002: Overlaps with the gateway's own session handling.
