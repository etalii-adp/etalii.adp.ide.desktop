<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->
<!-- last: ITEM-0002 REQ-0005 -->
# Orders

## Requirements

- ITEM-0001 Ordering #shop
  - REQ-0001 A customer orders without signing in. #guest #checkout
  - REQ-0002 An order is confirmed by e-mail. #mail state:proposed
  - REQ-0003 An order can be cancelled within an hour. category:technical state:accepted
  - REQ-0004 Orders are kept for seven years. category:legal category:technical #archive
- ITEM-0002 Delivery
  - REQ-0005 A parcel is tracked from the warehouse to the door. category:Technical
