workspace "Keyword prefixes" {
    model {
        grouped = person "Grouped" "Its identifier starts with group."
        components = softwareSystem "Components" "Its identifier starts with component." {
            persons = container "Persons" "Its identifier starts with person." "Go"
        }
        GROUP "A group" {
            modelled = container "Modelled" "Its identifier starts with model." "Go"
        }
    }
}
