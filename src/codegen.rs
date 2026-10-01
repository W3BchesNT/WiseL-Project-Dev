use crate::parser::ASTNode;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetPlatform {
    Windows,
    Linux,
    Mac, // in future
}

pub fn generate(ast: &[ASTNode]) -> String {
    let mut asm = String::new();

    let mut format_str = String::new();
    for node in ast {
        if let ASTNode::Format(f) = node {
            format_str = f.clone();
        }
    }

    if !format_str.is_empty() {
        asm.push_str(&format!("format {}\n", format_str));
    }

    asm.push_str("entry start\n");
    asm.push('\n');

    for node in ast {
        if let ASTNode::IncludeInc(inc) = node {
            asm.push_str(&format!("include '{}'\n\n", inc));
        }
    }

    // Define target platform
    let fmt_lower = format_str.to_lowercase();
    let target = if fmt_lower.contains("elf64") {
        TargetPlatform::Linux
    } else if fmt_lower.contains("macho64") || fmt_lower.contains("mac") {
        TargetPlatform::Mac
    } else {
        TargetPlatform::Windows // По умолчанию — уважаемая классика
    };

    match target {
        TargetPlatform::Windows => {
            // === WINDOWS (PE64) ===
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

            asm.push_str("section '.idata' import data readable writeable\n");
            
            let mut libs: Vec<(String, Vec<String>)> = Vec::new();
            for node in ast {
                if let ASTNode::Library { name, functions } = node {
                    libs.push((name.clone(), functions.clone()));
                }
            }

            let lib_entries: Vec<String> = libs.iter().map(|(lib, _)| format!("{}, '{}'", lib, format!("{}.DLL", lib).to_uppercase())).collect();
            if !lib_entries.is_empty() {
                asm.push_str(&format!("    library {}\n", lib_entries.join(", \\\n       ")));
            }

            for (lib, funcs) in &libs {
                if funcs.is_empty() { continue; }
                let func_imports: Vec<String> = funcs.iter().map(|f| format!("{}, '{}'", f, f)).collect();
                asm.push_str(&format!("    import {}, \\\n           {}\n", lib, func_imports.join(", \\\n           ")));
            }
        }

        TargetPlatform::Linux => {
            // === LINUX (ELF64) ===
            for node in ast {
                if let ASTNode::Data(content) = node {
                    asm.push_str("segment readable writeable\n");
                    for line in content.lines() {
                        if !line.trim().is_empty() {
                            asm.push_str(&format!("    {}\n", line.trim()));
                        }
                    }
                    asm.push('\n');
                }
            }

            asm.push_str("segment readable executable\n\n");
            asm.push_str("start:\n");

            for node in ast {
                if let ASTNode::Asm(code) = node {
                    for line in code.lines() {
                        asm.push_str(&format!("    {}\n", line.trim()));
                    }
                }
            }
        }

        TargetPlatform::Mac => {
            // === MAC (Mach-O 64) — in future ===
            asm.push_str("; TODO: Mac OS (Mach-O) code generation\n");
        }
    }

    asm
}