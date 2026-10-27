workspace "Hierarchical identifiers" {
    !identifiers hierarchical
    model {
        u = person "User" "A user."
        a = softwareSystem "A" "First." {
            api = container "API" "A's API." "Go"
        }
        b = softwareSystem "B" "Second." {
            api = container "API" "B's API." "Go"
        }
        u -> a.api "Calls A" "HTTPS"
        u -> b.api "Calls B" "HTTPS"
        u -> api "Calls whichever" "HTTPS"
    }
    views {
        container a "a" {
            include *
        }
        container b "b" {
            include *
        }
    }
}
