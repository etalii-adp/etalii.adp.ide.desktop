# 1. Record architecture decisions

Date: 2026-02-11

## Status

Accepted

## Context

The model in `courier.dsl` says what the architecture *is*. It does not say why any of it is that
way, and the reasoning is what a reader six months later actually needs.

## Decision

We record architecture decisions as short markdown files in this folder, numbered in the order
they were taken, and point at the folder with `!adrs decisions` in the model.

## Consequences

The decisions travel with the model in the same repository, reviewed in the same pull requests.
A decision that is superseded is not deleted — it gets a `Superseded by` status, so the record
shows what was believed at the time.
