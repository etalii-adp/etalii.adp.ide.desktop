workspace "View statements" {
    model {
        u = person "User" "A user."
        s = softwareSystem "System" "A system." {
            web = container "Web" "Serves." "React"
            api = container "API" "Answers." "Kotlin"
            db = container "Store" "Keeps." "PostgreSQL"
        }
        u -> web "Visits" "HTTPS"
        web -> api "Calls" "JSON"
        api -> db "Reads" "SQL"
    }
    views {
        container s "titled" {
            title "A title the document declares"
            include *
            exclude db
            autoLayout tb 300 150
        }
        container s "named" {
            include web api
            include u
        }
        dynamic s "steps" {
            title "Numbered steps"
            u -> web "Visits"
            web -> api "Calls"
            api -> db
            autoLayout lr
        }
    }
}
