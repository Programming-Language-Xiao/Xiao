use std::{env, fs};
use xiao_source::SourceFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in env::args().skip(1) {
        let source = SourceFile::new(fs::read_to_string(&path)?);
        let result = xiao_syntax::parse(&source);
        if !result.diagnostics.is_empty() {
            eprintln!("{path}: {:#?}", result.diagnostics);
            std::process::exit(1);
        }
        println!("{path}: diagnostics=0");
    }
    Ok(())
}
