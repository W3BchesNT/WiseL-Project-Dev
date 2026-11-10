mod codegen;
mod input;
mod lexer;
mod lexer_helpers;
mod parser;
mod parser_helpers;
mod parser_pratt;

// "use kawaii::*;"
use codegen::*;
use input::*;
use lexer::*;
use parser::*;
use std::fs;
use std::process::Command;
use std::path::PathBuf;

fn get_wiselc_root() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    exe.parent().unwrap().parent().unwrap().parent().unwrap().to_path_buf()
}

fn handle_cli(args: &[String]) -> bool {
    if args.len() > 1 && args[1] == "new" {
        if args.len() < 3 {
            eprintln!("[ERROR] Usage: wisel new <project_name>");
            std::process::exit(1);
        }
        let project_name = &args[2];
        std::fs::create_dir_all(project_name).unwrap();

        let main_wise = format!("{}/main.wise", project_name);
        
        let starter_code = r#"Target "Windows"
UseLib "library/stdlib.wlib"

fn main() {
    @target="Windows" {
        WindowsShadowSpace()
    }
    print_str("Hello, World!")
    Exit()
}
"#;
        
        std::fs::write(&main_wise, starter_code).unwrap();

        println!("Creater project '{}'", project_name);
        println!("Run: cd {} && wisel main.wise", project_name);
        return true;
    }

    if args.len() > 1 && args[1] == "build" {
        if args.len() < 3 {
            eprintln!("[ERROR] Usage: wisel build <file.wise>");
            std::process::exit(1);
        }
        
        let user_path = PathBuf::from(&args[2]);
        
        let absolute_path = if user_path.is_absolute() {
            user_path
        } else {
            std::env::current_dir().unwrap().join(&user_path)
        };

        let original_dir = std::env::current_dir().unwrap();
        let wiselc_root = get_wiselc_root();
        
        std::env::set_current_dir(&wiselc_root).unwrap();
        
        compile_file(absolute_path.to_str().unwrap());
        
        println!("[*.*] Running assembler...");
        
        let output = Command::new("cmd")
            .args(["/C", "fasmgc-outbuild.bat"])
            .output()
            .expect("Failed to execute fasmgc-outbuild.bat");

        if output.status.success() {
            println!("[*.*] Moving artifacts to project folder...");
            
            let out_asm_src = wiselc_root.join("out.asm");
            let out_asm_dst = original_dir.join("out.asm");
            if let Err(e) = fs::copy(&out_asm_src, &out_asm_dst) {
                eprintln!("[ERROR] Failed to copy out.asm: {}", e);
            }

            let main_exe_src = wiselc_root.join("main.exe");
            let main_exe_dst = original_dir.join("main.exe");
            if let Err(e) = fs::copy(&main_exe_src, &main_exe_dst) {
                eprintln!("[ERROR] Failed to copy main.exe: {}", e);
            }

            //let _ = fs::remove_file(&out_asm_src);
            //let _ = fs::remove_file(&main_exe_src);
        }

        std::env::set_current_dir(original_dir).unwrap();

        if !output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!("{}", String::from_utf8_lossy(&output.stderr));
            eprintln!("[ERROR] Assembler failed.");
            std::process::exit(1);
        }
        return true;
    }

    false
}

fn compile_file(path: &str) {
    let source: String = read_file(path);
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

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if handle_cli(&args) {
        return;
    }

    let path: String = get_entry_path();
    compile_file(&path);
}