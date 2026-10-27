workspace "Tags in a block" {
    model {
        u = person "User" "A user." "Internal"
        ext = softwareSystem "Partner" "Someone else's system." {
            tags "External"
        }
        s = softwareSystem "System" "A system." "Core" {
            db = container "Store" "Keeps things." "PostgreSQL" {
                tags "Database" "Existing System"
            }
            tags "Extra, More"
        }
        u -> s "Uses" "HTTPS"
        s -> ext "Calls" "HTTPS"
        u -> db "Reads" "SQL"
    }
    views {
        systemContext s "context" {
            include *
        }
        container s "containers" {
            include *
        }
    }
}
