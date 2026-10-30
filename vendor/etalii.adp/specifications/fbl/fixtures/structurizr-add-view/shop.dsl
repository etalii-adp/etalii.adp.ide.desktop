workspace "Shop" {
    model {
        shop = softwareSystem "Shop" "Sells things."
    }
    views {
        systemContext shop "context" {
            include *
        }
    }
}
