workspace "Sidecar keys" {
    model {
        alpha = softwareSystem "Alpha" "First."
        beta = softwareSystem "Beta" "Second."
        alpha -> beta "Calls" "HTTPS"
    }
    views {
        systemLandscape "Landscape" {
            include *
        }
    }
}
