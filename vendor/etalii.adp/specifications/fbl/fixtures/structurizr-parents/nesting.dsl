workspace "Parents" {
    model {
        p = person "User" "A user." {
            inside = container "Inside" "Under a person." "Go"
        }
        s = softwareSystem "System" "A system." {
            api = container "API" "Answers." "Go" {
                db = component "Store" "Keeps." "SQL"
            }
        }
        group "A group" {
            loose = container "Loose" "In a group." "Go"
        }
    }
}
