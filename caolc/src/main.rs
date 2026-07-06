use std::{
    fs::read_to_string,
    io::{BufReader, Read, stdin},
    path::PathBuf,
};

use clap::Parser;
use clap_derive::Parser;

#[derive(Debug, Parser)]
struct Args {
    /// Input .caol script file. If omitted, then stdin is used
    #[arg()]
    file: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let input = match args.file {
        Some(path) => read_to_string(path).expect("Failed to read script file"),
        None => {
            let mut code = String::new();
            BufReader::new(stdin())
                .read_to_string(&mut code)
                .expect("Failed to read code");
            code
        }
    };

    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_cao_lang::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("Error loading CaoLang parser");
    let tree = parser.parse(input, None).unwrap();
    tree.print_dot_graph(&std::io::stdout());
}
