# 1. Exactly one container may cross into the control network

Date: 2026-03-04

## Status

Accepted

## Context

Several services would find it convenient to read a tag directly: OEE wants line rate, Quality
wants fill volumes, Batch Execution wants to know when the last pallet left. The obvious thing is
to let each one open its own OPC UA session to the SCADA server.

The control network is a safety boundary, not a performance boundary. Every additional process
that can reach it is another thing that could write to a tag, exhaust a session pool on a PLC, or
be the reason an auditor asks how a filler was commanded to change speed at 03:14.

## Decision

Exactly one container — `OPC UA Gateway` — holds a session to the control network. It runs in the
plant DMZ. The industrial firewall permits OPC UA outbound from that host and nothing else, and
permits no inbound connection at all.

Everything else consumes line data by subscribing to the plant event bus, which the gateway
publishes to.

## Consequences

- One place to audit, one place to rate-limit, one place where a recipe download can be authorised.
- Line data is available to every service, but a fraction of a second later than the tag changed.
  Nothing in this system needs it sooner; anything that did would belong in the PLC, not here.
- The gateway is a single point of failure for line visibility, so it runs active/standby on two
  hosts. Losing both stops data collection, not production — the line keeps filling.
- A new service that wants tag data adds a consumer, not a firewall rule. That is the point.
