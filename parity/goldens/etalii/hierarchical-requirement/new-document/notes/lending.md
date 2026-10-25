<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0007 REQ-0009 -->
# lending

## How to read this file

This file holds the requirements of one subject, kept by people and agents together. Leave its first two lines as they are.

- Under `Requirements` each bullet is an item (`ITEM-`), a part of the subject, or a requirement (`REQ-`), which states one requirement. An item holds requirements and further items, indented under it. A requirement stands in an item and holds nothing.
- A number is never changed and never used again. Add an entry with `ITEM-NEW` or `REQ-NEW` in place of a number: the designer numbers it.
- Lines indented under a bullet that are not bullets describe it. A description holds no bullets.
- A bullet's first line ends with its tags (`#name`), and a requirement's with one category (`category:name`) and one state (`state:name`) after them. A name has no spaces. A title does not end in a word that starts with `#`, `category:` or `state:`.
- The categories are functional, non-functional and technical, and the states proposed, accepted, rejected and done; others can be added.
- Refer to an entry by its number in square brackets: `[REQ-0001]`.
- Under `Assessments` each bullet is a question, remark, warning or idea about one entry, named by its number: `- [ ] Question REQ-0001: text`. Write `[x]` to resolve one. Lines indented under it are its detail.

```markdown
- ITEM-NEW Checkout #core
  What a customer does between the basket and the receipt.
  - REQ-NEW An order is confirmed by e-mail. #mail category:functional state:proposed
    Sent once the payment of [REQ-0001] succeeds.

- [ ] Question REQ-0001: Which payment methods count?
- [x] Idea ITEM-0001: Offer a guest checkout.
```

## Requirements

- ITEM-0001 User interface
  - REQ-0001 Every page can be used on a phone, a tablet and a desktop screen.
  - REQ-0002 Every action can be taken with the keyboard alone.
  - ITEM-0002 Accessibility
    - REQ-0003 Text and controls meet WCAG 2.2 at level AA.
- ITEM-0003 Integration
  - REQ-0004 Every external service is reached through one documented interface.
- ITEM-0004 Backend
  - REQ-0005 Every request is answered within two seconds.
  - REQ-0006 A failed request is logged with what is needed to repeat it.
  - ITEM-0005 Security
    - REQ-0007 A user signs in before seeing or changing their own data.
- ITEM-0006 Data
  - REQ-0008 Personal data is kept no longer than it is needed.
- ITEM-0007 Processes
  - REQ-0009 A release can be rolled back within one hour.

## Assessments
