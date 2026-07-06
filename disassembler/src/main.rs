use cao_lang::{compiler, prelude::CaoProgram};
use clap::Parser;

#[derive(Debug, clap_derive::Parser)]
struct Args {
    #[arg()]
    format: Format,
}

#[derive(Debug, clap_derive::ValueEnum, Clone, Default)]
enum Format {
    #[default]
    Json,
}

fn main() {
    let args = Args::parse();

    match args.format {
        Format::Json => {
            let reader = std::io::BufReader::new(std::io::stdin().lock());
            let pl: CaoProgram = serde_json::from_reader(reader).expect("Failed to deserialize");
            let compiled = compiler::compile(pl, None).expect("Failed to compile");

            compiled.print_disassembly();
        }
    }
}
