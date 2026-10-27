workspace "Block comments" {
    model {
        /* before */ t = softwareSystem "Other" "Another system." /* after */
        u = person "User" "A user." /* an inline block comment */
        /*
        hidden = person "Hidden" "Declared inside a block comment."
        u -> hidden "Inside the comment" {
        */
        s = softwareSystem "System" "A system." /* a brace { inside a comment */
        s -> t "Calls" /* between */ "HTTPS"
        u -> s "Uses" "HTTPS"
    }
    views {
        systemLandscape "landscape" {
            include *
        }
    }
}
