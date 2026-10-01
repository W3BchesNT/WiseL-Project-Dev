use crate::parser::ASTNode;

pub fn generate(ast: &[ASTNode]) -> String {
    let mut asm = String::new();

    let mut format_str = None;
    for node in ast {
        if let ASTNode::Format(f) = node {
            format_str = Some(f);
        }
    }

    if let Some(fmt) = format_str {
        asm.push_str(&format!("format {}\n", fmt));
    }
    
    asm.push_str("entry start\n\n");
    asm.push_str("include 'win64w.inc'\n\n");

    for node in ast {
        if let ASTNode::Data(content) = node {
            asm.push_str("section '.data' data readable writeable\n");
            for line in content.lines() {
                if !line.trim().is_empty() {
                    asm.push_str(&format!("    {}\n", line.trim()));
                }
            }
            asm.push('\n');
        }
    }

    asm.push_str("section '.text' code readable executable\n\n");
    asm.push_str("start:\n");

    for node in ast {
        if let ASTNode::Asm(code) = node {
            for line in code.lines() {
                asm.push_str(&format!("    {}\n", line.trim()));
            }
        }
    }

    asm.push_str("    sub rsp, 8\n");
    asm.push_str("    and rsp, -16\n");
    asm.push_str("    invoke ExitProcess, 0\n\n");

    asm.push_str("section '.idata' import data readable writeable\n");
    
    let mut libs: Vec<(String, Vec<String>)> = Vec::new();
    for node in ast {
        if let ASTNode::Library { name, functions } = node {
            libs.push((name.clone(), functions.clone()));
        }
    }

    //let libs_names
    let lib_entries: Vec<String> = libs.iter().map(|(lib, _)| format!("{}, '{}'", lib, format!("{}.DLL", lib).to_uppercase())).collect();
    asm.push_str(&format!("    library {}\n", lib_entries.join(", \\\n       ")));

    for (lib, funcs) in &libs {
        if funcs.is_empty() { continue; }
        let func_imports: Vec<String> = funcs.iter().map(|f| format!("{}, '{}'", f, f)).collect();
        asm.push_str(&format!("    import {}, \\\n           {}\n", lib, func_imports.join(", \\\n           ")));
    }

    asm
}