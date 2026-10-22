# Northgate Bottling MES

The MES turns a works order from SAP into a filled, capped, labelled and palletised batch, and
records what actually happened well enough that Quality can release it and Finance can confirm it.

## Read the deployment view first

Most systems can be understood from the container view. This one cannot, because the constraint
that shapes it does not appear there: **the control network is isolated, and only one container is
allowed to cross into it.**

The plant runs on the Purdue model. Levels 1–2 — the PLCs and SCADA that physically move the
machines — sit on an air-gapped VLAN. There is no route from the enterprise network to it. The
only path is the plant DMZ, where the OPC UA Gateway runs, and the industrial firewall permits
exactly one thing: OPC UA outbound, initiated by the gateway. Nothing initiates inbound. Ever.

That is why `OPC UA Gateway` is drawn in its own colour, and why it is the only container with a
line to `SCADA and PLC Layer`.

## Why SCADA is not a container

`SCADA and PLC Layer` is modelled as a separate software system, not as a container of the MES.
That is deliberate and occasionally argued about:

- It is nobody's deployable unit of the MES. It is Ignition and a rack of Siemens S7s.
- It predates this system by a decade and will outlive it.
- Drawing it inside the MES boundary would claim the MES team owns and can change it. They do not
  and cannot; changing ladder logic is a validated change with a safety case attached.

The C4 rule of thumb applies: a container is something *you* deploy. Draw everything else as the
external system it is.

## Availability, not uptime

The line does not stop because the internet does. Everything the line needs to keep running is in
the plant data centre, on premises. The only things that may fail without stopping production are
the ERP confirmations, which queue, and the LIMS results, which hold a batch in quarantine rather
than stopping the fillers.
