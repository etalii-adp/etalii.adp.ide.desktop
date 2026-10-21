workspace "containers" {

    model {
        containers = softwareSystem "containers"
    }

    views {
        container containers "containers" {
            include *
            autoLayout lr
        }
    }

}
