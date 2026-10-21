workspace "context" {

    model {
        context = softwareSystem "context"
    }

    views {
        systemContext context "context" {
            include *
            autoLayout lr
        }
    }

}
