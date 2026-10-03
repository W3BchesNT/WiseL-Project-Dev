pub enum Step {
    READ      = 1,
    TOKENIZED = 2,
    PARSED    = 3,
    GENERATED = 4,
}

pub fn print_step(step: Step, size: usize) {
    let message: String = match step {
        Step::READ      => format!("Read main.wise:         {size} bytes"),
        Step::TOKENIZED => format!("Tokenized:              {size} tokens"),
        Step::PARSED    => format!("Parsed:                 {size} AST nodes"),
        Step::GENERATED => format!("Generated out.asm:      {size} bytes\n[*.*] {}",
                                   "Run:                    fasm.exe out.asm main.exe && main.exe"
        ),
    };
    let current = step as usize;

    println!("[{current}/4] {message}");
}

pub fn get_entry_path() -> String {
    std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("main.wise"))
}

pub fn read_file(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error)  => {
            eprintln!("[ERROR] Cannot read {path}: {error}");
            std::process::exit(1);
        }
    }
}