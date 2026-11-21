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
use std::time::Instant;

fn get_wiselc_root() -> PathBuf {
    let mut current = std::env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Executable has no parent directory")
        .to_path_buf();

    loop {
        if current.join("fasmgc-outbuild.bat").exists() || current.join("Cargo.toml").exists() {
            return current;
        }

        if !current.pop() {
            eprintln!("[ERROR] Could not find wisel root directory (missing fasmgc-outbuild.bat or Cargo.toml)");
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!("Hello, I am WiseL! The compiler for the WiseL language.");
    println!();
    println!("Usage:");
    println!("  wisel new <name>           Create a new WiseL project");
    println!("  wisel build [file]         Build the project (default: main.wise)");
    println!("  wisel run                  Run the existing main.exe without building");
    println!("  wisel br [file] [flags]    Force build and run");
    println!("  wisel help                 Show this help message");
    println!("  wisel <file>               Compile a specific file");
    println!();
    println!("Flags for 'br':");
    println!("  --benchmark                Measure build and run time");
}

fn handle_cli(args: &[String]) -> bool {
    if args.len() == 1 {
        print_help();
        return true;
    }

    if args[1] == "help" || args[1] == "-h" || args[1] == "--help" {
        print_help();
        return true;
    }

    if args[1] == "new" {
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

        println!("Created project '{}'", project_name);
        println!("Run: cd {} && wisel run", project_name);
        return true;
    }

    if args[1] == "run" {
        let exe_path = std::env::current_dir().unwrap().join("main.exe");
        
        if !exe_path.exists() {
            eprintln!("[ERROR] main.exe not found. Run 'wisel build' first.");
            std::process::exit(1);
        }

        println!("[*.*] Running main.exe...");
        let status = Command::new(&exe_path)
            .status()
            .expect("Failed to run main.exe");
        
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        return true;
    }

    if args[1] == "build" || args[1] == "build-run" || args[1] == "br" {
        
        let benchmark = args.iter().any(|a| a == "--benchmark");
        let file_name = if args.len() >= 3 && !args[2].starts_with("--") { &args[2] } else { "main.wise" };
        let user_path = PathBuf::from(file_name);
        
        let absolute_path = if user_path.is_absolute() {
            user_path
        } else {
            std::env::current_dir().unwrap().join(&user_path)
        };

        let original_dir = std::env::current_dir().unwrap();
        let wiselc_root = get_wiselc_root();
        
        let exe_path = original_dir.join("main.exe");
        let is_build_run = args[1] == "build-run" || args[1] == "br";
        let start_compile = Instant::now();

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
        }

        std::env::set_current_dir(&original_dir).unwrap();

        if !output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!("{}", String::from_utf8_lossy(&output.stderr));
            eprintln!("[ERROR] Assembler failed.");
            std::process::exit(1);
        }

        if benchmark {
            let elapsed = start_compile.elapsed();
            println!("[BENCHMARK] Build time: {:.2?}", elapsed);
        }

        if is_build_run {
            println!("[*.*] Running main.exe...");
            
            let start_run = Instant::now();
            let status = Command::new(&exe_path)
                .status()
                .expect("Failed to run main.exe");
            
            if benchmark {
                let elapsed = start_run.elapsed();
                println!("[BENCHMARK] Run time: {:.2?}", elapsed);
            }

            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
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