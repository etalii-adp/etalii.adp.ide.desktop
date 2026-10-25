workspace "Trailing comments" { // after the workspace brace
    model { # after the model brace
        u = person "User" "A user." // after a whole declaration
        s = softwareSystem "System" "A system." { // after an element's brace
            web = container "Web" { # a container with a block and nothing else
            }
            api = container "API" "Serves." // no technology yet
        }
        t = softwareSystem "Partner" // no description yet
        h = softwareSystem "Hash # inside" "See http://example.com//path" # marks inside quotes
        u -> s "Uses" // no technology yet
        s -> t "Calls" "HTTPS" # after a whole relationship
    }
    views { // after the views brace
        systemLandscape "landscape" { // after a view's brace
            include * // after include
        }
        container s "containers" {
            include * # after include
            autoLayout lr // after autoLayout
        }
    }
} // after the closing brace
