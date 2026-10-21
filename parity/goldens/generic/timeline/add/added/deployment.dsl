workspace "deployment" {

    model {
        deployment = softwareSystem "deployment"

        deploymentEnvironment "Production" {
            deploymentNode "Server" {
            }
        }
    }

    views {
        deployment deployment "Production" "deployment" {
            include *
            autoLayout lr
        }
    }

}
