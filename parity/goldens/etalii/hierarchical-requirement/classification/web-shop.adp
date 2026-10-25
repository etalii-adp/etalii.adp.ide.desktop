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
  technical: brown
  functional: blue
  quality: purple
  legal: pink
states:
  accepted: green
  rejected: red
  proposed: gray
  done: purple
  in-review: default
collapsed:
  ITEM-0003: yes
  REQ-0002: yes
filters:
  Open security: #security @proposed sort:state
  Backend: item:back* sort:number
