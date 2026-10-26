workspace "Misplaced elements" {
    model {
        s = softwareSystem "System" "A system."
        stray = container "Stray" "Written straight in the model." "Go"
        loose = component "Loose" "A component outside any container." "Go"
        p = person "User" "A user." {
            inside = container "Inside a person" "Under the wrong parent." "Go"
        }
        group "A group" {
            grouped = container "Grouped" "In a group at the top of the model." "Go"
        }
        p -> stray "Calls" "HTTPS"
        stray -> loose "Uses" "In process"
    }
    views {
        systemLandscape "landscape" {
            include *
        }
        container s "containers" {
            include stray inside grouped
        }
    }
}
