workspace "Unidentified elements" {
    model {
        person "User" "Declared without an identifier."
        softwareSystem "System" "Also without one." {
            container "Web" "Serves." "React"
        }
        named = softwareSystem "Named" "Has an identifier."
        softwareSystem "System" "A second of the same name."
        person "" "No name either."
    }
    views {
        systemLandscape "landscape" {
            include *
        }
    }
}
