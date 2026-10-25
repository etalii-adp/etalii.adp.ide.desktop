<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0007 REQ-0013 -->
# Web shop

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

The requirements of the web shop, from what a customer sees to how the shop is run.

- ITEM-0001 Storefront #customer
  What a customer sees and does in the shop.
  * REQ-0001 A customer finds a product by its name or its description. #search category:functional state:accepted
  * REQ-0002 Every page loads within two seconds on a mobile connection. #performance category:non-functional state:proposed
    Measured at the 95th percentile, on a 4G connection.
  * ITEM-0002 Checkout #payment
    From the basket to the confirmation of the order.
    - ITEM-0003 Guest checkout
      + REQ-0005 A customer can order without an account. state:rejected
      + REQ-0013 A guest is offered an account after the order. #customer category:functional
- ITEM-0004 Backend #core
  Everything behind the public interface.
  - REQ-0006 A session ends after 30 minutes without activity. #security category:functional state:accepted
    Activity is any request the customer makes. See [REQ-0007] for signing in.
  - REQ-0007 Customers sign in with an e-mail address and a password. #security category:functional state:accepted
  - ITEM-0005 Mail
    - REQ-0008 Mail goes through one provider, and its outage does not stop an order. category:technical state:proposed
      Its outage is shown on [ITEM-0005], and [REQ-0004] waits for it.
    - REQ-0004 An order is confirmed by e-mail once its payment succeeds. #mail category:functional state:done
      Sent when [REQ-0003] succeeds, through the service of [ITEM-0005].
- ITEM-0006 Data
  - REQ-0009 Order data is kept for seven years, as tax law requires. #legal category:non-functional state:accepted
  - REQ-0010 Personal data is removed on request within 30 days. #legal #privacy category:non-functional
    See [REQ-0099](https://example.com/privacy), a link and no reference.
    Requests reach [REQ-0009] first.

## Assessments

- [ ] Question REQ-0006: Is 30 minutes right for the mobile app?
- [x] Warning ITEM-0002: Overlaps with the payment provider's own session handling.
  Resolved: the provider's session ends before ours.
- [ ] remark REQ-0003: Two customers asked for Klarna.
- Idea ITEM-0006: Move orders older than a year to cheaper storage.
