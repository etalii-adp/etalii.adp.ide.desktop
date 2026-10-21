workspace "Line Endings" {

    model {
        u = person "User"
        s = softwareSystem "System"
        u -> s "Uses"
    }

    views {
        systemContext s "context" {
            include *
        }
    }

}
