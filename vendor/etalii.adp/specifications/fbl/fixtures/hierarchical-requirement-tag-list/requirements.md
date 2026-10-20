<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0003 REQ-0005 -->
# Accounts

## Requirements

- ITEM-0001 Sign-in #security
  - REQ-0001 A user signs in with an e-mail address and a password. #security category:functional state:accepted
  - REQ-0002 A password has at least twelve characters. category:non-functional state:proposed
  - ITEM-0002 Sessions
    - REQ-0003 A session ends after 30 minutes without activity. #Security #ux
- ITEM-0003 Profile
  - REQ-0004 A user changes their own e-mail address. #UX #self-service state:proposed
  - REQ-0005 A user deletes their account. state:rejected #privacy
