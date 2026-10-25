etalii/hierarchical-requirement
body: web-shop.md
tags:
  customer: blue
  search: default
  performance: orange
  payment: green
  mail: yellow
  core: default
  security: red
  legal: purple
  privacy: pink
categories:
  functional: blue
  non-functional: purple
  technical: brown
states:
  proposed: gray
  accepted: green
  rejected: red
  done: blue
collapsed:
  ITEM-0003: yes
  REQ-0002: yes
filters:
  Security, open: #security @proposed sort:state
  Backend: item:back* sort:title
  Done: state:done sort:title
