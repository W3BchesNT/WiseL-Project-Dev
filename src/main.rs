mod codegen;
mod input;
mod lexer;
mod parser;

// "use kawaii::*;"
use codegen::*;
use input::*;
use lexer::*;
use parser::*;
use std::fs;

fn main() {
    let path: String = get_entry_path();
    let source: String = read_file(&path);
    print_step(Step::READ, source.len());

    let tokens: Vec<Token> = tokenize(&source);
    print_step(Step::TOKENIZED, tokens.len());

    let ast: Vec<ASTNode> = parse(&tokens);
    print_step(Step::PARSED, ast.len());

    let asm: String = generate(&ast);
    if let Err(e) = fs::write("out.asm", &asm) {
        eprintln!("[ERROR] Cannot write out.asm: {e}");
        std::process::exit(1);
    }
    print_step(Step::GENERATED, asm.len());
}