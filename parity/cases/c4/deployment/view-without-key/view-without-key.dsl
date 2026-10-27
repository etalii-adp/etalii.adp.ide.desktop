workspace "Views without keys" {
    model {
        u = person "User" "A user."
        s = softwareSystem "System" "A system." {
            web = container "Web" "Serves." "React" {
                c = component "Controller" "Handles." "Spring"
            }
        }
        u -> c "Uses" "HTTPS"
        deploymentEnvironment "Live" {
            deploymentNode "Server" "A box." "Linux" {
                containerInstance web
            }
        }
    }
    views {
        systemLandscape {
            include *
        }
        systemContext s {
            include *
        }
        container s "containers" "A key and a description." {
            include *
        }
        component web {
            include *
        }
        dynamic s {
            u -> c "Uses"
        }
        deployment s "Live" {
            include *
        }
        deployment * "Live" "everywhere" "Every system, live." {
            include *
        }
    }
}
