workspace "landscape" {

    model {
        landscape = softwareSystem "landscape"
    }

    views {
        systemLandscape "landscape" {
            include *
            autoLayout lr
        }
    }

}
