<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0006 REQ-0011 -->
# Library

The requirements of the town library's lending system.

## How to read this file

Leave the first two lines as they are.

```markdown
- ITEM-NEW An example in a fence, never read
  - REQ-NEW Nor this one.
```

## Requirements

From the counter to the catalogue. This paragraph is the author's, and is never read.

- ITEM-0001 Lending #core
  Everything between a member and the books they borrow.

  Lending at every branch.
  - REQ-0001 A member borrows at most ten books at once. #limits category:functional state:accepted
    Counted over every branch, see [ITEM-0004].
  - REQ-0002 A loan lasts three weeks. state:proposed category:functional category:technical
  * ITEM-0002 Renewals
    A renewal extends a loan by three weeks.
    * REQ-0003 A loan is renewed twice at most. #limits #renewals
    * ITEM-0003 Online renewals #web
      + REQ-0004 A member renews a loan on the website.
      + REQ-0005 A renewal is refused while the book is reserved. category:functional
        The member is told that someone is waiting, without their name.
  The author's note after the nested items: part of ITEM-0001's span, not its description.

### Catalogue and accounts

- ITEM-0004 Catalogue
	- REQ-0006 Every book can be found by its title, author or ISBN. #search state:done
	- ITEM-0005 Search results
	  - REQ-0007 Results are sorted by relevance.
		Relevance weighs the title above the author.
- ITEM-0006 Accounts category:admin #security
  - REQ-0008 A member signs in with a library card number and a PIN. #security
    See the [card rules](https://example.org/cards).

## Assessments

## Notes

- ITEM-0099 Outside both sections, so not read and not reported.
