workspace "Lenient spellings" {
    MODEL {
        User = Person User "Written without quotes around its name."
        sys = SoftwareSystem "System" "Keywords in any case." {
            web = Container "Web"
            api = container "API" "Serves." "Kotlin" "Tag1,Tag2" extra words
        }
        DeploymentEnvironment "Live" {
            node = deploymentNode "Server" "A box." "Linux" "" 4 more
        }
        user -> sys "Uses"
        web->api "Written without spaces around the arrow"
        User -> SYS "Ends in another case"
        user -> web Unquoted
    }
    VIEWS {
        SystemContext sys "Context" {
            INCLUDE *
        }
        container sys containers {
            include *
        }
    }
}
