# Courier Tracking

Courier Tracking books parcel deliveries, follows them from pickup to doorstep, and tells the
sender where their parcel has got to. It is used directly by senders and couriers, and by support
agents when a delivery goes wrong.

## Why the scan pipeline is asynchronous

A courier scans a parcel in a van, on a handset, on whatever signal the street has. The scan has
to be accepted quickly and cannot wait on a mail server. So `Scan Ingest` records the scan and
publishes an event; `Notification Sender` picks it up afterwards and mails the sender. If mail is
slow or down, scanning keeps working and the notifications catch up.

That is the reason the container view has an event broker in it, and the reason the
`ParcelScanned` dynamic view is worth reading: the ordering is the design.

## What lives where

| Concern | Container |
|---|---|
| Booking and tracking over HTTP | `API Application` |
| Anything a browser renders | `Single-Page Application`, served by `Web Application` |
| Anything a courier does in the field | `Courier App` |
| Durable record of deliveries, parcels, scans | `Delivery Database` |
| Decoupling scans from notifications | `Event Broker` |

## About this document

`!docs docs` in `courier.dsl` points at this folder. ADP round-trips that line and this file
untouched — it has no documentation model of its own yet — while Structurizr renders this
alongside the diagrams. Both facts are the point of including it in a reference example.
