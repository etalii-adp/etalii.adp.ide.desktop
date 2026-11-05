workspace "Sourceless" {
    model {
        u = person "User" "A user." {
            -> s "Uses" "HTTPS"
        }
        s = softwareSystem "System" "A system." {
            web = container "Web" "Serves." "React" {
                -> api "Calls" "JSON"
            }
            api = container "API" "Answers." "Kotlin"
        }
        person "Nameless" "Has no identifier." {
            -> s "Uses"
        }
        u -> web "Visits"
    }
}
