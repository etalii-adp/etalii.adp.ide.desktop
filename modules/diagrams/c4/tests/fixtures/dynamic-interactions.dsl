workspace "Dynamic" "Ordered interactions, including the nested form." {

    model {
        u = person "User"
        s = softwareSystem "System" {
            web = container "Web App" "" "React"
            api = container "API" "" "Kotlin"
            db = container "Database" "" "PostgreSQL"
        }
        u -> web "Submits the form"
        web -> api "Posts to" "JSON/HTTPS"
        api -> db "Writes" "SQL/TCP"
        api -> web "Responds" "JSON/HTTPS"
    }

    views {
        dynamic s "signup" "A user signs up." {
            u -> web "Submits the form"
            web -> api "Posts to"
            api -> db "Writes"
            api -> web "Responds"
            autoLayout lr
        }
    }

}
