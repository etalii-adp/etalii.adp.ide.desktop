workspace "Finding order" {
    !include shared/people.dsl
    model {
        p = person "Person"
        s = softwareSystem "System" "A system." {
            c = container "Container" "A container."
        }
        loose = component "Loose" "Outside a container." "Go"
        q = person "Other person" "Connected."
        q -> c
        q -> missing "Dangling" "HTTPS"
        live = deploymentEnvironment "Live" {
            n = deploymentNode "Node" {
                gw = infrastructureNode "Gateway"
                ci = containerInstance c
            }
        }
    }
    views {
        systemContext nowhere "lost" {
            include *
        }
        systemContext s "context" {
            include c
        }
        container s "nothing" {
            exclude c
        }
        deployment s "Live" "live" {
            include *
        }
    }
}
