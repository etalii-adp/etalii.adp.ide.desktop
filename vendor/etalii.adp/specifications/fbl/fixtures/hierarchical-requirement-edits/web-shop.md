<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0003 REQ-0006 -->
# Web shop

## Requirements

- ITEM-0001 Backend #core
  Everything behind the public interface.
  - REQ-0001 A session ends after 30 minutes without activity. #security category:functional state:accepted
    Activity is any request the user makes. See [REQ-0002] for sign-in.
  - REQ-0002 Users sign in with an e-mail address and a password. #security
  - ITEM-0002 Sessions
    - REQ-0003 Tokens are refreshed without the user noticing.
    - REQ-0002 A session can be ended from every device at once. state:proposed
- ITEM-0003 Data
  - REQ-0004 Personal data is kept no longer than it is needed. category:non-functional

## Assessments

- [ ] Question REQ-0001: Is 30 minutes right for the mobile app?
