etalii/hierarchical-requirement
body: web-shop.md
tags:
  legal: purple
  customer: blue
  privacy: pink
  search: default
  performance: orange
  payment: green
  mail: yellow
  core: orange
  infosec: red
  vendor-lock: default
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
  Open security: #security @proposed sort:state
  Backend: item:back* sort:number
