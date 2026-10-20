<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0003 REQ-0004 -->
# Payments

## Requirements

- ITEM-0001 Checkout
  How a customer pays. The card rules are in [REQ-0002].
  - REQ-0001 A customer pays by card or by bank transfer, as [REQ-0099] says. #payment category:functional
    The provider's terms are on [REQ-0099](https://example.com/terms), a link and no reference.
  - ITEM-0002 Refunds
    - REQ-0003 A refund goes back to the card it came from.
- ITEM-0003 Reporting
  - REQ-0004 Every payment is listed in the daily report.

## Assessments

- [ ] Question REQ-0002: Is ten seconds right when [REQ-0003] applies?
- [ ] Remark ITEM-0002: Partial refunds come later.
  They wait for [REQ-0002] and for [ITEM-0003].
