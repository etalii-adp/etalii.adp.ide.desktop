workspace "Shop" {
    model {
        customer = person "Customer" "Buys things."
        shop = softwareSystem "Shop" "Sells things."
        customer -> shop "Buys from"
        !include extras.dsl
    }
    views {
        container shop "containers" {
            include *
            autoLayout lr
        }
    }
}
