workspace "Unbalanced" {
    model {
        u = person "User" "A user."
        s = softwareSystem "System" "A system." {
            web = container "Web" "Serves." "React"
        u -> web "Visits" "HTTPS"
    }
    views {
        container s "containers" {
            include *
        }
