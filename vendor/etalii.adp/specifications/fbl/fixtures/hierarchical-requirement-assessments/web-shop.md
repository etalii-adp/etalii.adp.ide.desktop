<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0003 REQ-0006 -->
# Web shop

## Requirements

- ITEM-0001 Backend #core
  - REQ-0001 A session ends after 30 minutes without activity. #security category:functional
  - REQ-0002 Users sign in with an e-mail address and a password. #security
  - ITEM-0002 Sessions
    - REQ-0003 Tokens are refreshed without the user noticing.
- ITEM-0003 Data
  Everything the shop keeps about its customers and their orders.

## Assessments

- [ ] Question REQ-0001: Is 30 minutes right for the mobile app?
- [x] warning ITEM-0002: Overlaps with the gateway's own session handling.
  The gateway's session ends first, so ours never times out alone.
- REMARK REQ-0002: Two customers asked to sign in with a passkey.
- [X] idea ITEM-0003: Split the data into orders and customers.
- [ ] Question REQ-0006: Is a failed payment retried?
