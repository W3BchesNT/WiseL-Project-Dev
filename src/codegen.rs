use crate::parser::ASTNode;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetPlatform {
    Windows,
    Linux,
}

// push_code_block:
fn push_cb(asm: &mut String, raw_content: &str) {
    let trimmer = raw_content.trim();

    let cleaned = if let Some(pos) = trimmer.find('{') {
    let after_open = &trimmer[pos + 1..];
        if let Some(end_pos) = after_open.rfind('}') {
            &after_open[..end_pos]
        } else {
            after_open
        }
    } else {
        trimmer
    };

    for line in cleaned.lines() {
        if !line.trim().is_empty() {
            asm.push_str(&format!("    {}\n", line.trim()));
        }
    }
}

pub fn generate(ast: &[ASTNode]) -> String {
    let mut asm = String::new();
    let mut format_str = String::new();

    for node in ast {
        if let ASTNode::Format(f) = node {
            format_str = f.clone();
        }
    }
    
    for node in ast {
        if let ASTNode::Asm(code) = node {
            push_cb(&mut asm, code);
        }
    }

    asm.push_str("include 'kernel/FASM/fasmg/extension/format.inc'\n");
    asm.push_str("include 'kernel/FASM/fasmg/extension/wir_ll.inc'\n\n");
    asm.push_str("set_build X64, Windows, Console\n\n");
    asm.push_str("entry start\n\n");

    for node in ast {
        if let ASTNode::IncludeInc(inc) = node {
            asm.push_str(&format!("include '{}'\n\n", inc));
        }
    }

    // target
    let fmt_lower = format_str.to_lowercase();
    let target = if fmt_lower.contains("elf64") {
        TargetPlatform::Linux
    } else {
        TargetPlatform::Windows
    };

    for node in ast {
        if let ASTNode::Data(content) = node {
            asm.push_str("data_block\n");
            for line in content.lines() {
                if !line.trim().is_empty() {
                    asm.push_str(&format!("    {}\n", line.trim()));
                }
            }
            asm.push('\n');
        }
    }

    asm.push_str("code_block\n\n");
    asm.push_str("start:\n");

    for node in ast {
        if let ASTNode::Asm(code) = node {
            push_cb(&mut asm, code);
        }
    }
    asm.push('\n');

    for node in ast {
        if let ASTNode::Func { name, body } = node {
            if name != "main" {
                asm.push_str(&format!("{}:\n", name));
            }
            push_cb(&mut asm, body);
        }
    }
    asm.push('\n');

    if target == TargetPlatform::Windows {
        asm.push_str("import_block\n");

        let mut libs: Vec<(String, Vec<String>)> = Vec::new();
        for node in ast {
            if let ASTNode::Library { name, functions } = node {
                libs.push((name.clone(), functions.clone()));
            }
        }

        let lib_entries: Vec<String> = libs
            .iter()
            .map(|(lib, _)| format!("{}, '{}'", lib, format!("{}.DLL", lib).to_uppercase()))
            .collect();

        if !lib_entries.is_empty() {
            asm.push_str(&format!("    library {}\n", lib_entries.join(", \\\n       ")));
        }

        for (lib, funcs) in &libs {
            if funcs.is_empty() {
                continue;
            }
            let func_imports: Vec<String> = funcs.iter().map(|f| format!("{}, '{}'", f, f)).collect();
            asm.push_str(&format!("    import {}, \\\n           {}\n", lib, func_imports.join(", \\\n           ")));
        }
    }

    asm
}