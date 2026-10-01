mod codegen;
mod input;
mod lexer;
mod native;
mod parser;
mod __llvm_codegen;

// "use kawaii::*;"
use codegen::*;
use input::*;
use lexer::*;
use parser::*;
use std::fs;

fn main() {
    let options: Options = get_options();
    let source: String = read_file(&options.input);
    print_step(Step::Read, source.len(), &options);

    let tokens: Vec<Token> = tokenize(&source);
    print_step(Step::Tokenized, tokens.len(), &options);

    let ast: Vec<ASTNode> = parse(&tokens);
    print_step(Step::Parsed, ast.len(), &options);

    // /*
    if !options.__llvm {
        let asm: String = generate(&ast);
        if let Err(e) = fs::write("out.asm", &asm) {
            eprintln!("[ERROR] Cannot write out.asm: {e}");
            std::process::exit(1);
        }
        print_step(Step::Generated, asm.len(), &options);

        return;
    }
    // */

    // /*
    let ir: String = __llvm_codegen::generate(&ast).unwrap();
    let output_filename: std::path::Display = options.output.display();
    let generated: Vec<u8> = if options.run {
        native::generate_executable(&ir).unwrap()
    } else {
        ir.into_bytes()
    };

    fs::write(&options.output, &generated)
        .map_err(|error| format!("[ERROR] Cannot write {output_filename}: {error}"))
        .unwrap();
    print_step(Step::Generated, generated.len(), &options);

    if options.run {
        let executable = fs::canonicalize(&options.output).unwrap();
        let status = std::process::Command::new(executable)
            .status()
            .map_err(|error| format!("[ERROR] Cannot run {output_filename}: {error}"))
            .unwrap();
        let exit_code = status.code().unwrap();

        print_step(Step::Run, exit_code as usize, &options);
    }
    // */
}
