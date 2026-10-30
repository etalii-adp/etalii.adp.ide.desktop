workspace "Absent arguments" {
    model {
        s = softwareSystem "S" {
            web = container "Web" // c
            api = container "API" ""
        }
    }
}
