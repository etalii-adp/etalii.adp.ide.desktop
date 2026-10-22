# 2. The plant runs when the corporate network does not

Date: 2026-03-19

## Status

Accepted

## Context

The first architecture put the execution services in the corporate cloud and left only the gateway
on site. It was cheaper to run and easier to deploy. Then a fibre cut took the site off the WAN
for four hours, and all three lines stopped — not because the machines could not fill bottles, but
because no operator could be told which order to run next.

Downtime on a bottling line is measured in thousands of units per hour. The saving did not survive
the first incident.

## Decision

Everything an operator needs to keep a line running lives in the plant data centre, on premises:
the execution services, the screens they serve, the event bus and the databases.

Only two integrations may depend on the WAN, and both degrade rather than stop:

| Integration | Behaviour when the WAN is down |
|---|---|
| ERP confirmations | Queue on the event bus and post when the link returns |
| LIMS sample results | The batch stays in quarantine; filling continues |

## Consequences

- There is a data centre on site to run, patch and cool. Accepted.
- Deployments are to a plant Kubernetes cluster with a maintenance window bounded by the
  production schedule, not by office hours.
- The plant database has a synchronous standby in the second comms room, because "on premises"
  moves the failure mode from the WAN to the building.
- Corporate reporting reads from a replica that syncs when it can, and is explicitly allowed to be
  behind. Nobody on the floor waits for it.
