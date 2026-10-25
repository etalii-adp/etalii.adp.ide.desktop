/*
 * Courier Tracking - the reference model for ADP's C4 support.
 *
 * One document, several registrations. Every construct ADP's parser understands appears here at
 * least once, so this file doubles as a worked example and as a legible statement of what is and
 * is not supported. Comments in every position are deliberate: the writer preserves them.
 */

workspace "Courier Tracking" "Parcel tracking for senders, couriers and support staff." {

    model {
        # Both comment styles are legal on a line of their own, and both survive an edit.

        // `flat` is the default; stated so the choice is visible. ADP resolves identifiers
        // flat, so `hierarchical` would change what a view scope names and is deliberately
        // not used here.
        !identifiers flat

        // ---- people ----------------------------------------------------------------------
        sender = person "Sender" "Books a delivery and follows where the parcel is."
        courier = person "Courier" "Collects and delivers parcels, and scans them on the way."
        agent = person "Support Agent" "Answers questions about deliveries that went wrong." "Staff"

        // ---- the system in scope ---------------------------------------------------------
        tracking = softwareSystem "Courier Tracking" "Books deliveries, tracks parcels and tells everyone where they are." {

            // Prose and decisions, in folders beside this file. ADP does not read either - it
            // has no documentation model yet - but it round-trips both untouched, and
            // Structurizr renders them alongside the diagrams.
            !docs docs
            !adrs decisions

            // A group is a label over elements rather than a container. It survives the round
            // trip and is drawn as a boundary by tools that support it.
            group "Customer facing" {
                web = container "Web Application" "Serves the single-page application to a browser." "Java and Spring Boot"
                spa = container "Single-Page Application" "Books deliveries and shows the progress of a parcel." "TypeScript and React"
                mobile = container "Courier App" "Scans parcels at pickup, handover and delivery." "Kotlin and Jetpack Compose"
            }

            group "Behind the API" {
                api = container "API Application" "The booking and tracking API." "Java and Spring Boot" {
                    // Components live inside a container, never beside one.
                    bookingController = component "Booking Controller" "Takes a booking and turns it into a delivery." "Spring MVC Controller"
                    trackingController = component "Tracking Controller" "Answers where a parcel has got to." "Spring MVC Controller"
                    scanIngest = component "Scan Ingest" "Accepts scans from couriers and orders them by time." "Spring Component"
                    notifier = component "Notification Sender" "Tells a sender when their parcel moves." "Spring Component"
                    routingFacade = component "Routing Facade" "Talks to the routing optimiser." "Spring Component"

                    // In-process calls. ADP does not require a technology here - C4 itself
                    // leaves such calls bare - but Structurizr inspects for one on every
                    // relationship, and this model is written to pass that too.
                    bookingController -> routingFacade "Asks for a route" "Method call"
                    scanIngest -> notifier "Raises a parcel moved event" "Method call"
                }

                broker = container "Event Broker" "Carries scan events so ingest and notification are not one step." "Apache Kafka" "Queue"
                database = container "Delivery Database" "Stores deliveries, parcels and every scan against them." "PostgreSQL 16" "Database"
            }
        }

        // ---- systems this one depends on -------------------------------------------------
        // "Existing System" is what C4 calls something already there; ADP draws it muted.
        routing = softwareSystem "Routing Optimiser" "Works out the cheapest route for a set of parcels." "Existing System"
        email = softwareSystem "E-mail System" "The corporate mail relay." "Existing System"
        maps = softwareSystem "Mapping Provider" "Turns addresses into coordinates and draws maps." "Existing System"

        // ---- relationships ---------------------------------------------------------------
        // Person to system, at context level.
        sender -> tracking "Books deliveries and tracks parcels using" "HTTPS"
        courier -> tracking "Scans parcels using" "HTTPS"
        agent -> tracking "Looks up deliveries using" "HTTPS"

        // System to system, with the protocol named.
        tracking -> routing "Requests routes from" "JSON/HTTPS"
        tracking -> email "Sends mail through" "SMTP"
        tracking -> maps "Geocodes addresses using" "JSON/HTTPS"
        email -> sender "Delivers mail to" "SMTP"

        // Person to container, which is what a container view draws.
        sender -> spa "Books and tracks using" "HTTPS"
        courier -> mobile "Scans parcels using" "Touch"
        agent -> spa "Looks up deliveries using" "HTTPS"

        // Container to container. Every one names its protocol - this is the level where that
        // matters, and where ADP warns when it is missing.
        web -> spa "Delivers to the web browser of the customer" "HTTPS"
        spa -> api "Calls" "JSON/HTTPS"
        mobile -> api "Calls" "JSON/HTTPS"
        api -> database "Reads from and writes to" "JDBC/SSL"
        api -> broker "Publishes scan events to" "Kafka protocol"
        broker -> api "Delivers scan events to" "Kafka protocol"
        api -> routing "Requests routes from" "JSON/HTTPS"
        api -> email "Sends mail through" "SMTP"
        api -> maps "Geocodes addresses using" "JSON/HTTPS"

        // Component level, including calls that leave the container.
        spa -> bookingController "Books a delivery through" "JSON/HTTPS"
        spa -> trackingController "Asks where a parcel is through" "JSON/HTTPS"
        mobile -> scanIngest "Posts scans to" "JSON/HTTPS"
        bookingController -> database "Writes the delivery to" "JDBC/SSL"
        trackingController -> database "Reads scans from" "JDBC/SSL"
        scanIngest -> broker "Publishes to" "Kafka protocol"
        notifier -> email "Sends through" "SMTP"
        routingFacade -> routing "Calls" "JSON/HTTPS"

        // ---- where it runs ---------------------------------------------------------------
        production = deploymentEnvironment "Production" {

            deploymentNode "Sender device" "A laptop or a phone." "Browser" {
                deploymentNode "Web Browser" "Whatever the sender already has." "Chrome, Safari or Firefox" {
                    containerInstance spa
                }
            }

            deploymentNode "Courier handset" "Issued with the round." "Android 14" {
                containerInstance mobile
            }

            deploymentNode "Regional data centre" "The racks of the operator." "Ubuntu 24.04 LTS" {

                // An instance count sits *after* the tags, which is the argument order a
                // line-based parser most often gets backwards.
                deploymentNode "web-*" "Web tier" "Ubuntu 24.04 LTS" "" 3 {
                    deploymentNode "Apache Tomcat" "Servlet container." "Tomcat 10" {
                        containerInstance web
                        containerInstance api
                    }
                }

                loadBalancer = infrastructureNode "Load Balancer" "Terminates TLS and spreads traffic across the web tier." "nginx"

                deploymentNode "kafka-*" "Event brokers." "Ubuntu 24.04 LTS" "" 3 {
                    containerInstance broker
                }

                // Named so they can be related below. Replication is a property of the servers,
                // not of the two container instances on them - a relationship between instances
                // is refused, because instances inherit the relationships of what they deploy.
                dbPrimary = deploymentNode "db-primary" "Primary database server." "Ubuntu 24.04 LTS" {
                    containerInstance database
                }

                dbStandby = deploymentNode "db-standby" "Hot standby." "Ubuntu 24.04 LTS" {
                    containerInstance database
                }

                dbPrimary -> dbStandby "Replicates to" "Streaming replication"
            }

            // Relationships between deployment elements are declared in the environment.
            loadBalancer -> web "Forwards requests to" "HTTPS"
        }
    }

    views {
        systemLandscape "Landscape" "Every system in the delivery estate." {
            include *
            autoLayout tb
        }

        systemContext tracking "SystemContext" "Courier Tracking in its world." {
            include *
            autoLayout
        }

        container tracking "Containers" "What Courier Tracking is made of." {
            include *
            autoLayout lr
        }

        component api "ApiComponents" "Inside the API application." {
            include *
            // An exclude after an include is how one element is taken back out.
            exclude maps
            autoLayout lr
        }

        dynamic tracking "ParcelScanned" "What happens when a courier scans a parcel." {
            mobile -> api "Posts the scan"
            api -> database "Records the scan against the parcel"
            api -> broker "Publishes a parcel moved event"
            broker -> api "Delivers the event to the notifier"
            api -> email "Sends the sender an update"
            email -> sender "Arrives as mail"
            autoLayout lr
        }

        deployment tracking "Production" "ProductionDeployment" "Where Production runs." {
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
            element "Staff" {
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
            element "Existing System" {
                background #999999
                color #ffffff
            }
            element "Infrastructure Node" {
                background #ffffff
                color #000000
            }
        }
    }

    // Not read by ADP either, and kept verbatim like everything else it does not model.
    configuration {
        scope softwaresystem
    }

}
