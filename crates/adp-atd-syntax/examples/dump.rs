fn main() {
    let path = std::env::args().nth(1).unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    let parse = adp_atd_syntax::parse(&text);
    for t in &parse.tokens {
        if t.kind != adp_atd_syntax::TokenKind::Trivia {
            println!("{:?} {} {:?}", t.kind, t.name(), t.text(&text));
        }
    }
    println!("errors: {:?}", parse.errors);
    println!("root: {}", parse.root.is_some());
}
