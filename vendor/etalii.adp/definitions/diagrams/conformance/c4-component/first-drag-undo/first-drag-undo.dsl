workspace "First drag" {
    model {
        a = softwareSystem "A" "First."
        b = softwareSystem "B" "Second."
        a -> b "Calls" "HTTPS"
    }
    views {
        systemLandscape "one" {
            include *
        }
        systemLandscape "two" {
            include *
        }
    }
}
