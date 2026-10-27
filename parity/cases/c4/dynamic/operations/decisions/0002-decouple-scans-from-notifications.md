# 2. Decouple scans from notifications with an event broker

Date: 2026-02-18

## Status

Accepted

## Context

A courier scans a parcel on a handset, in a van, on whatever mobile signal the street happens to
have. The first implementation had `Scan Ingest` write the scan and send the sender an e-mail in
the same request. Two problems followed from that:

- When the mail relay was slow, scanning was slow. Couriers waited at the door for a spinner.
- When the mail relay was down, scans failed outright, so the parcel's history got holes in it —
  the one thing the system exists to keep.

## Decision

`Scan Ingest` records the scan and publishes a *parcel moved* event to an event broker. A separate
`Notification Sender` consumes those events and sends the mail.

## Consequences

Scanning no longer depends on the mail relay being up, which is the point. In exchange:

- Notifications are eventually consistent. A sender may refresh the tracking page and see a scan
  before the e-mail about it arrives; this is acceptable and the page is the faster path anyway.
- There is a broker to run, and it appears in the container and deployment views as three
  instances rather than one, because losing it would stall every notification.
- The ordering now matters enough to be worth drawing, which is what the `ParcelScanned` dynamic
  view is for.
