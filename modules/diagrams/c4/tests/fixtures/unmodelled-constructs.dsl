workspace "Unmodelled" "Constructs ADP does not model but must never drop." {

    !identifiers hierarchical

    model {
        u = person "User"
        s = softwareSystem "System" {
            web = container "Web" "" "React"
            !docs docs
            !adrs adrs
        }
        u -> s "Uses"
    }

    views {
        systemContext s "context" {
            include *
        }

        styles {
            element "Person" {
                shape Person
                background #08427b
                color #ffffff
            }
            element "External" {
                background #999999
                color #ffffff
            }
        }

        theme default
    }

    configuration {
        scope softwaresystem
    }

}
