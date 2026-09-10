fn main() {
    let cases = vec![
        "* whole line comment\nregress y x",
        "regress wage education experience female, robust",
        "regress y x // trailing comment\nsummarize y // other",
        "did y, treat(treated) time(post) cluster(firm)",
        "ivregress 2sls wage (education = distance), robust",
        "xtreg wage education, fe entity(firm)",
    ];
    for c in cases {
        match numeris_command::lexer::lex(c) {
            Ok(t) => println!(
                "{:?} => {:?}",
                c,
                t.iter().map(|x| &x.tok).collect::<Vec<_>>()
            ),
            Err(e) => println!("{:?} => ERR {}", c, e.what),
        }
    }
}
