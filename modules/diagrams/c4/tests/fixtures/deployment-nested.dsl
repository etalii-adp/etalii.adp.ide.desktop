workspace "Deployment" "Nested deployment nodes, instances and infrastructure nodes." {

    model {
        s = softwareSystem "System" {
            web = container "Web App" "" "React"
            api = container "API" "" "Kotlin"
            db = container "Database" "" "PostgreSQL"
        }

        production = deploymentEnvironment "Production" {
            region = deploymentNode "eu-west-1" "" "Region" {
                dns = infrastructureNode "Route 53" "" "DNS"
                lb = infrastructureNode "Load Balancer" "" "ELB"

                cluster = deploymentNode "Cluster" "" "ECS" {
                    node = deploymentNode "Service" "" "Fargate" {
                        containerInstance web
                        containerInstance api
                    }
                }

                data = deploymentNode "Data Tier" "" "RDS" {
                    primary = deploymentNode "Primary" "" "PostgreSQL" {
                        containerInstance db
                    }
                    replica = deploymentNode "Replica" "" "PostgreSQL" {
                        containerInstance db
                    }
                }

                dns -> lb "Forwards to" "HTTPS"
            }
        }
    }

    views {
        deployment s "Production" "production" {
            include *
            autoLayout lr
        }
    }

}
