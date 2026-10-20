<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0005 REQ-0008 -->
# The library's catalogue (bibliothèque)

## Requirements

- ITEM-0001 Search #core
  Finding a book in the catalogue.
  - REQ-0001 A book is found by its title. #search category:functional
    Matched ignoring case.
  - REQ-0002 A book is found by its author. category:functional
  - ITEM-0002 Filters
    - REQ-0003 Results can be narrowed to one branch.
- ITEM-0003 Lending
  Loans and their limits.
  * REQ-0004 A member borrows at most ten books at once. state:accepted
    Counted over every branch.
  * REQ-0006 A loan lasts three weeks.
    Counted from the day the book is collected.

    A renewal starts a new loan.
- ITEM-0004 Accounts #security
  + REQ-0007 A member signs in with a card number and a PIN.

## Assessments
