workspace "Tags" {
    model {
        s = softwareSystem "System" "A system." "Core" {
            tags "Extra" "More"
            web = container "Web"
            tags "Last, Least"
        }
    }
}
