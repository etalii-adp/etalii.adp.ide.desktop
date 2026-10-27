workspace "Dangling relationships" {
    model {
        u = person "User" "A user."
        s = softwareSystem "System" "A system."
        u -> s "Uses" "HTTPS"
        ghost -> phantom
        u -> phantom
        ghost -> s "Labelled" "HTTPS"
        u -> s
    }
    views {
        systemContext s "context" {
            include *
        }
    }
}
