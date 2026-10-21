/*
 * The model a layout defect was found on, kept because nothing simpler reproduced it.
 *
 * A copy of the industrial-plant example with its component view left as `include *`, which
 * puts every container in the plant outside a boundary drawn around one of them. Several
 * elements then leave that boundary through the same edge and land on exactly the same spot -
 * and boxes that are exactly coincident cannot be separated by nudging pairs apart, because
 * identical geometry means both compute the same escape and move together forever.
 *
 * The only edit against the example it was copied from: `!adrs` points at this folder`s own
 * adrs directory rather than the example`s decisions one, so the document still validates here.
 */
/*
 * Northgate Bottling - Manufacturing Execution System.
 *
 * A works-order-to-pallet system for a beverage bottling plant: three filling lines, a
 * warehouse, and the corporate ERP above it all.
 *
 * The shape of this model is driven by one thing that does not appear in an office system - the
 * OT/IT split. Everything below the DMZ is plant equipment on an isolated control network,
 * speaks industrial protocols, and cannot be restarted because a deployment is due. Everything
 * above it is ordinary software. The gateway between them is the most important box here, and
 * the deployment view is the one an engineer will actually argue about.
 */

workspace "Northgate Bottling MES" "Manufacturing execution for the Northgate bottling plant." {

    model {
        !identifiers flat

        // ---- the people on and around the plant floor -------------------------------------
        operator = person "Line Operator" "Runs a filling line, confirms setup, and reports downtime against a reason code."
        supervisor = person "Shift Supervisor" "Sequences works orders across the three lines and signs off shift handover."
        quality = person "Quality Engineer" "Reviews in-process checks and releases or quarantines a batch."
        maintainer = person "Maintenance Technician" "Responds to breakdowns and records what was done."
        planner = person "Production Planner" "Turns demand into a weekly plan in the ERP." "Office"

        // ---- the system being described ----------------------------------------------------
        mes = softwareSystem "Manufacturing Execution System" "Turns works orders into executed batches, and records what actually happened on the line." {

            !docs docs
            !adrs adrs

            group "Operations floor" {
                hmi = container "Line HMI" "The screen at the line: current order, setup confirmation, downtime reasons, in-process checks." "TypeScript and React, kiosk browser"
                andon = container "Andon Display" "Wall-mounted board showing line state, rate against target, and open calls." "TypeScript and React"
            }

            group "Execution services" {
                orderService = container "Order Service" "Receives works orders, sequences them per line, and holds the state of each." "Java and Spring Boot" {
                    orderIntake = component "Order Intake" "Accepts works orders from the ERP and validates them against the line catalogue." "Spring Component"
                    sequencer = component "Line Sequencer" "Decides which order a line runs next, honouring changeover rules." "Spring Component"
                    orderState = component "Order State Machine" "Released, staged, running, held, complete - and the transitions allowed between them." "Spring Component"
                    materialCheck = component "Material Availability" "Asks the warehouse whether the bill of materials can be satisfied." "Spring Component"
                    orderRepository = component "Order Repository" "Reads and writes works orders and their state." "Spring Data JPA"

                    orderIntake -> orderState "Creates the order in" "Method call"
                    sequencer -> orderState "Advances" "Method call"
                    sequencer -> materialCheck "Asks whether the order can run" "Method call"
                    orderState -> orderRepository "Persists through" "Method call"
                    orderIntake -> orderRepository "Persists through" "Method call"
                }

                batchService = container "Batch Execution Service" "Follows a batch from first fill to last pallet, and holds the electronic batch record." "Java and Spring Boot"
                qualityService = container "Quality Service" "In-process checks, sampling plans, and the release or quarantine decision." "Java and Spring Boot"
                oeeService = container "OEE Service" "Availability, performance and quality per line, per shift." "Python and FastAPI"
            }

            group "Plant integration" {
                opcGateway = container "OPC UA Gateway" "The only thing that talks to the control network: subscribes to line tags and issues recipe downloads." "C# and OPC UA SDK" "Gateway"
                erpAdapter = container "ERP Adapter" "Pulls works orders down and posts confirmations back." "Java and Spring Boot"
                historianBridge = container "Historian Bridge" "Streams tag values into the process historian and reads them back for analysis." "C# and .NET"
            }

            group "Stores" {
                operationalDb = container "Operational Database" "Works orders, batches, checks, downtime events." "PostgreSQL 16" "Database"
                batchArchive = container "Batch Record Archive" "Signed, immutable electronic batch records kept for the retention period." "PostgreSQL 16" "Database"
                eventBus = container "Plant Event Bus" "Carries line events so slow consumers cannot stall the line." "Apache Kafka" "Queue"
            }
        }

        // ---- everything else on and around the plant ---------------------------------------
        // The control layer. It is a software system in its own right and deliberately NOT a
        // container of the MES: it is nobody's deployable unit, it long predates this system,
        // and drawing it inside would claim the MES team owns it.
        scada = softwareSystem "SCADA and PLC Layer" "Ignition SCADA and the Siemens S7 PLCs that actually run the fillers, cappers and palletisers." "Existing System, OT"
        historian = softwareSystem "Process Historian" "Long-term tag history for every sensor on the plant." "Existing System, OT"

        erp = softwareSystem "SAP S/4HANA" "Demand, works orders, stock and financials." "Existing System"
        lims = softwareSystem "LIMS" "Laboratory results for samples taken off the line." "Existing System"
        cmms = softwareSystem "CMMS" "Maintenance work orders and asset history." "Existing System"
        warehouse = softwareSystem "Warehouse Management System" "Where the materials are and what has been picked." "Existing System"

        // ---- people to systems --------------------------------------------------------------
        operator -> mes "Runs the line using" "HTTPS"
        supervisor -> mes "Sequences orders and signs off shifts using" "HTTPS"
        quality -> mes "Releases or quarantines batches using" "HTTPS"
        maintainer -> mes "Records downtime causes using" "HTTPS"
        planner -> erp "Plans production in" "SAP GUI"

        // ---- system to system -----------------------------------------------------------------
        mes -> erp "Receives works orders from and confirms production back to" "IDoc over HTTPS"
        mes -> scada "Subscribes to line tags and downloads recipes to" "OPC UA"
        mes -> historian "Writes tag history to and reads it back from" "OPC UA HDA"
        mes -> lims "Requests sample results from" "REST/HTTPS"
        mes -> cmms "Raises maintenance work orders in" "REST/HTTPS"
        mes -> warehouse "Checks material availability against" "REST/HTTPS"
        scada -> historian "Logs tag values to" "OPC UA"

        // ---- people to containers ---------------------------------------------------------
        operator -> hmi "Confirms setup and reports downtime on" "HTTPS"
        operator -> andon "Reads line state from" "HTTPS"
        supervisor -> hmi "Sequences the line on" "HTTPS"
        quality -> hmi "Records in-process checks on" "HTTPS"
        maintainer -> andon "Sees open calls on" "HTTPS"

        // ---- container to container ---------------------------------------------------------
        hmi -> orderService "Reads the order queue and confirms setup through" "JSON/HTTPS"
        hmi -> batchService "Reports downtime and progress through" "JSON/HTTPS"
        hmi -> qualityService "Records in-process checks through" "JSON/HTTPS"
        andon -> oeeService "Polls line performance from" "JSON/HTTPS"

        orderService -> operationalDb "Reads from and writes to" "JDBC/SSL"
        orderService -> eventBus "Publishes order state changes to" "Kafka protocol"
        orderService -> warehouse "Checks material availability against" "REST/HTTPS"

        batchService -> operationalDb "Reads from and writes to" "JDBC/SSL"
        batchService -> batchArchive "Writes the signed batch record to" "JDBC/SSL"
        batchService -> eventBus "Publishes batch events to" "Kafka protocol"
        eventBus -> batchService "Delivers line events to" "Kafka protocol"

        qualityService -> operationalDb "Reads from and writes to" "JDBC/SSL"
        qualityService -> lims "Requests sample results from" "REST/HTTPS"
        qualityService -> batchService "Releases or quarantines the batch through" "JSON/HTTPS"

        oeeService -> eventBus "Consumes line events from" "Kafka protocol"
        oeeService -> operationalDb "Reads downtime and order data from" "JDBC/SSL"

        // The gateway is the only container with a line into the control network.
        opcGateway -> scada "Subscribes to tags on and downloads recipes to" "OPC UA over TCP 4840"
        opcGateway -> eventBus "Publishes line events to" "Kafka protocol"
        eventBus -> opcGateway "Delivers recipe download commands to" "Kafka protocol"

        historianBridge -> historian "Writes tag history to and reads it back from" "OPC UA HDA"
        historianBridge -> eventBus "Consumes line events from" "Kafka protocol"

        erpAdapter -> erp "Pulls works orders from and posts confirmations to" "IDoc over HTTPS"
        erpAdapter -> orderService "Hands works orders to" "JSON/HTTPS"
        batchService -> erpAdapter "Posts production confirmations through" "JSON/HTTPS"
        batchService -> cmms "Raises maintenance work orders in" "REST/HTTPS"

        // ---- component level ------------------------------------------------------------------
        erpAdapter -> orderIntake "Delivers works orders to" "JSON/HTTPS"
        hmi -> sequencer "Asks for the next order from" "JSON/HTTPS"
        orderRepository -> operationalDb "Reads from and writes to" "JDBC/SSL"
        orderState -> eventBus "Publishes state changes to" "Kafka protocol"
        materialCheck -> warehouse "Checks stock against" "REST/HTTPS"

        // ---- where it runs ----------------------------------------------------------------
        // Three zones, and the boundary between the first two is the point of the diagram.
        plant = deploymentEnvironment "Plant" {

            deploymentNode "Control network (Purdue level 1-2)" "Isolated. No routing to the enterprise network; the DMZ is the only way across." "Air-gapped VLAN" "OT" {

                deploymentNode "Filling line PLC" "One per line: filler, capper, labeller, palletiser." "Siemens S7-1500" "OT" 3 {
                    plcRuntime = infrastructureNode "PLC Runtime" "Executes the ladder logic that runs the machines." "Siemens TIA" "OT"
                }

                scadaServer = infrastructureNode "SCADA Server" "Ignition gateway, redundant pair." "Ignition 8.1" "OT"
                historianServer = infrastructureNode "Historian Server" "Tag history at one-second resolution." "AVEVA PI" "OT"
            }

            deploymentNode "Plant DMZ" "The only path between the control network and everything else." "Hardened VLAN" "DMZ" {
                firewall = infrastructureNode "Industrial Firewall" "Allows OPC UA outbound from the gateway only. Nothing initiates inbound." "Palo Alto"

                deploymentNode "gw-opc-*" "Gateway servers, active/standby." "Windows Server 2022" "" 2 {
                    containerInstance opcGateway
                    containerInstance historianBridge
                }
            }

            deploymentNode "Plant data centre" "On premises, because the line cannot stop when the internet does." "Ubuntu 24.04 LTS" {

                lineBalancer = infrastructureNode "Load Balancer" "Terminates TLS for everything on the floor." "HAProxy"

                deploymentNode "k8s-plant" "Plant Kubernetes cluster." "Kubernetes 1.31" {

                    deploymentNode "mes-services" "Namespace for the execution services." "Kubernetes namespace" "" 3 {
                        containerInstance orderService
                        containerInstance batchService
                        containerInstance qualityService
                        containerInstance oeeService
                        containerInstance erpAdapter
                    }

                    deploymentNode "mes-web" "Namespace for the operator-facing screens." "Kubernetes namespace" "" 2 {
                        containerInstance hmi
                        containerInstance andon
                    }
                }

                deploymentNode "kafka-plant-*" "Event bus brokers." "Ubuntu 24.04 LTS" "" 3 {
                    containerInstance eventBus
                }

                dbPrimary = deploymentNode "pg-primary" "Primary database server." "PostgreSQL 16 on Ubuntu 24.04 LTS" {
                    containerInstance operationalDb
                    containerInstance batchArchive
                }

                dbStandby = deploymentNode "pg-standby" "Synchronous standby in the second comms room." "PostgreSQL 16 on Ubuntu 24.04 LTS" {
                    containerInstance operationalDb
                    containerInstance batchArchive
                }

                dbPrimary -> dbStandby "Replicates synchronously to" "Streaming replication"
            }

            // Line-of-sight terminals, which are on the floor but are not control equipment.
            deploymentNode "Line HMI terminal" "One panel PC per line, plus one andon screen per hall." "Industrial panel PC, Chromium kiosk" "" 5 {
                containerInstance hmi
            }

            // Inside the control network, below anything the MES touches. Drawn because a
            // deployment view that stops at the firewall does not show what is being protected.
            scadaServer -> plcRuntime "Polls tags on and issues setpoints to" "Siemens S7 over Profinet"
            scadaServer -> historianServer "Logs every tag value to" "OPC UA"

            firewall -> scadaServer "Permits OPC UA to" "OPC UA over TCP 4840"
            lineBalancer -> firewall "Is not permitted through" "Blocked by policy"
        }
    }

    views {
        systemLandscape "Landscape" "Every system in the Northgate plant estate, OT and IT." {
            include *
            autoLayout tb
        }

        systemContext mes "SystemContext" "The MES in its world." {
            include *
            autoLayout
        }

        container mes "Containers" "What the MES is made of, and which part crosses into the control network." {
            include *
            autoLayout lr
        }

        component orderService "OrderServiceComponents" "Inside the Order Service." {
            include *
            autoLayout lr
        }

        dynamic mes "BatchRelease" "Releasing a finished batch, from last pallet to ERP confirmation." {
            opcGateway -> eventBus "Publishes the last-pallet event"
            eventBus -> batchService "Delivers the event"
            batchService -> qualityService "Asks whether all in-process checks passed"
            qualityService -> lims "Fetches the outstanding sample results"
            qualityService -> batchService "Confirms the batch may be released"
            batchService -> batchArchive "Writes the signed batch record"
            batchService -> erpAdapter "Hands over the production confirmation"
            erpAdapter -> erp "Posts the confirmation"
            autoLayout lr
        }

        deployment mes "Plant" "PlantDeployment" "Three zones, and the firewall that is the whole point." {
            include *
            autoLayout tb
        }

        styles {
            element "Element" {
                shape roundedbox
                background #1168bd
                color #ffffff
            }
            element "Person" {
                shape person
                background #08427b
                color #ffffff
            }
            element "Office" {
                background #6b6b6b
            }
            element "Container" {
                background #438dd5
            }
            element "Component" {
                background #85bbf0
                color #000000
            }
            element "Database" {
                shape cylinder
            }
            element "Queue" {
                shape pipe
            }
            element "Gateway" {
                background #d94f00
                color #ffffff
            }
            element "Existing System" {
                background #999999
                color #ffffff
            }
            // Anything on the control network, drawn so the OT/IT boundary is visible at a glance.
            element "OT" {
                background #b8860b
                color #ffffff
            }
            element "DMZ" {
                background #7a5c00
                color #ffffff
            }
            element "Infrastructure Node" {
                background #ffffff
                color #000000
            }
        }
    }

    configuration {
        scope softwaresystem
    }

}
