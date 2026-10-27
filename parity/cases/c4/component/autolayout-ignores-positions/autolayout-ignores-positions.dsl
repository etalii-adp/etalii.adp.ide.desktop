workspace "Auto layout" {
    model {
        a = softwareSystem "A" "First."
        b = softwareSystem "B" "Second."
        a -> b "Calls" "HTTPS"
    }
    views {
        systemLandscape "auto" {
            include *
            autoLayout lr 200 80
        }
        systemLandscape "manual" {
            include *
        }
    }
}
