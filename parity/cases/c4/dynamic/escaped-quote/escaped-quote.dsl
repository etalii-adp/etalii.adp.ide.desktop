workspace "Escaped \"quotes\"" {
    model {
        u = person "The \"user\"" "Says \"hello\"."
        s = softwareSystem "System" "A path C:\temp and a \"quote\"."
        u -> s "Says \"hi\"" "HTTP \"S\""
    }
    views {
        systemContext s "context" {
            include *
        }
    }
}
