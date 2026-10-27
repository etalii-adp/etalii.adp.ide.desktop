!include preamble.dsl
workspace "Includes" {
    !include https://example.com/model.dsl
    model {
        !include people.dsl
        u = person "User" "A user."
        s = softwareSystem "System" "A system."
        u -> s "Uses" "HTTPS"
    }
    views {
        !include views.dsl
        systemLandscape "landscape" {
            include *
        }
    }
}
