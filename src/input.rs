pub struct Options {
    pub input : std::path::PathBuf,
    pub output: std::path::PathBuf,
    pub run   : bool,
}

pub fn get_options() -> Options {
    let program_name = std::env::args()
        .nth(0)
        .unwrap_or_else(|| String::from("main.wise"));
    let mut input = None;
    let mut run = false;

    // The first argument is 'program_name', so we skip it
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--run" => run = true,
            _       => {
                if arg.ends_with(".wise") && input.is_none() {
                    input = Some(std::path::PathBuf::from(arg));
                } else {
                    eprintln!("[ERROR] Usage: {program_name} [source.wise] [--run]");
                    std::process::exit(1);
                }
            },
        }
    }

    Options {
        input : input.unwrap_or_else(|| "main.wise".into()),
        output: if run { "out.exe" } else { "out.ll" }.into(),
        run,
    }
}

pub enum Step {
    Read      = 1,
    Tokenized = 2,
    Parsed    = 3,
    Generated = 4,
    Run       = 5,
}

pub fn print_step(step: Step, info: usize, options: &Options) {
    let total_steps = if options.run { 5 } else { 4 };
    let message = match step {
        Step::Read      => format!("Read     : {info} bytes ({})", options.input.display()),
        Step::Tokenized => format!("Tokenized: {info} tokens"),
        Step::Parsed    => format!("Parsed   : {info} AST nodes"),
        Step::Generated => format!("Generated: {info} bytes ({})", options.output.display()),
        Step::Run       => format!("Exit code: {info}"),
    };
    let current_step = step as usize;

    println!("[{current_step}/{total_steps}] {message}");
}

pub fn read_file(path: &std::path::PathBuf) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error)  => {
            let path_str = path.to_str().unwrap_or_else(|| "Unknown path");

            eprintln!("[ERROR] Cannot read {path_str}: {error}");
            std::process::exit(1);
        }
    }
}
