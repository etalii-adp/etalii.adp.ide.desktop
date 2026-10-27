workspace "Nested relationships" {
    model {
        u = person "User" "A user." {
            -> s "Uses" "HTTPS"
        }
        s = softwareSystem "System" "A system." {
            web = container "Web" "Serves." "React" {
                -> api "Calls" "JSON/HTTPS"
            }
            api = container "API" "Answers." "Kotlin"
        }
        u -> web "Visits" "HTTPS" {
            tags "Async"
        }
        u -> nobody "Names nothing" "HTTPS"
        live = deploymentEnvironment "Live" {
            n = deploymentNode "Server" "A box." "Linux" {
                wi = containerInstance web
                ai = containerInstance api
                gw = infrastructureNode "Gateway" "Routes." "Nginx"
                gw -> wi "Forwards to" "HTTPS"
            }
            n -> gw "Hosts" "Process"
        }
    }
    views {
        systemContext s "context" {
            include *
        }
        container s "containers" {
            include *
        }
        deployment s "Live" "live" {
            include *
        }
    }
}
