// a line comment before the workspace
# a hash comment, which the DSL also accepts
/*
 * a block comment spanning lines
 */
workspace "Comments" "Every comment form the DSL actually accepts." {

    model { // trailing a brace is legal
        # hash comment inside the model
        // Trailing a declaration is NOT: the real parser reads the comment as extra
        // arguments and refuses the line. ADP accepts it anyway, which is harmless -
        // a reader that takes more than the writer emits - but the corpus must not
        // claim the format allows something it does not.
        u = person "User" "A user."
        /* block comment between elements */
        s = softwareSystem "System" "A system." {
            web = container "Web App" "Serves pages." "React"
            db = container "Database" "Stores things." "PostgreSQL"
            web -> db "Reads from and writes to" "SQL/TCP"
        }
        u -> web "Visits" "HTTPS"
    }

    views {
        systemContext s "context" {
            include *
        }
        container s "containers" {
            include *
        }
    }

}
// a trailing comment after the closing brace
