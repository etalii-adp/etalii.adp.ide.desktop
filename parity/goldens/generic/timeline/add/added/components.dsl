workspace "components" {

    model {
        components = softwareSystem "components" {
            application = container "Application" "" "Technology"
        }
    }

    views {
        component application "components" {
            include *
            autoLayout lr
        }
    }

}
