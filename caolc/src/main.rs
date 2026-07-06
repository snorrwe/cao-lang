use std::{
    fs::read_to_string,
    io::{BufReader, Read, stdin},
    path::PathBuf,
};

use cao_lang::{
    compiler::{CaoProgram, compile},
    traits::into_f1,
    value::Value,
    vm::Vm,
};
use clap::Parser;
use clap_derive::Parser;

#[derive(Debug, Parser)]
struct Args {
    /// Input .caol script file. If omitted, then stdin is used
    #[arg()]
    file: Option<PathBuf>,
}

fn make_vm() -> Vm<'static> {
    let mut vm = Vm::new(()).expect("Failed to init VM");
    vm.register_native_function(
        "print",
        into_f1(|_vm, v: Value| {
            println!("{v:?}");
            Ok(Value::Nil)
        }),
    )
    .expect("Failed to register print");
    vm
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
    let tree = parser
        .parse(input, None)
        .expect("Failed to parse caol script");
    tree.print_dot_graph(&std::io::stdout());

    // TODO: turn tree into cao-lang module
    let module: CaoProgram = todo!();

    let bytecode = compile(module, None).expect("Failed to compile program");

    let mut vm = make_vm();

    vm.run(&bytecode).expect("Runtime error");
}
