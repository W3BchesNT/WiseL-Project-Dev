use crate::parser::ASTNode;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetPlatform {
    Windows,
    Linux,
}

fn flatten_ast(ast: &[ASTNode], current_platform: TargetPlatform) -> Vec<ASTNode> {
    let mut result = Vec::new();
    for node in ast {
        match node {
            ASTNode::TargetBlock { target, nodes } => {
                let t_lower = target.to_lowercase();
                let matches = match current_platform {
                    TargetPlatform::Windows => t_lower.contains("win") || t_lower == "windows",
                    TargetPlatform::Linux => t_lower.contains("linux"),
                };
                if matches {
                    result.extend(flatten_ast(nodes, current_platform));
                }
            }
            _ => {
                result.push(node.clone());
            }
        }
    }
    result
}

fn push_func_body(asm: &mut String, raw_content: &str, user_funcs: &[String]) {
    let mut in_asm = false;
    let mut asm_brace_depth = 0;

    for line in raw_content.lines() {
        let norm_line = line.replace('\u{00A0}', " ");
        let trimmed = norm_line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if in_asm {
            asm_brace_depth += trimmed.matches('{').count() as i32;
            asm_brace_depth -= trimmed.matches('}').count() as i32;

            if asm_brace_depth <= 0 {
                in_asm = false;
                asm_brace_depth = 0;
            } else {
                asm.push_str(&format!("    {}\n", trimmed));
            }
            continue;
        }

        if trimmed.starts_with("asm") {
            in_asm = true;
            asm_brace_depth = trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
            if asm_brace_depth <= 0 {
                in_asm = false;
                asm_brace_depth = 0;
            }
            continue;
        }

        if trimmed == "{" || trimmed == "}" {
            continue;
        }

        let pure: String = trimmed.chars().filter(|c| !c.is_whitespace() && *c != '\u{00A0}').collect();
        if pure.ends_with("()") && pure.len() > 2 {
            let name = &pure[..pure.len() - 2];
            if !name.is_empty() && !name.contains('(') && !name.contains(')') {
                if user_funcs.contains(&name.to_string()) {
                    asm.push_str(&format!("    call fn_{}\n", name));
                } else {
                    asm.push_str(&format!("    call {}\n", name));
                }
                continue;
            }
        }

        asm.push_str(&format!("    {}\n", trimmed));
    }
}

pub fn generate(raw_ast: &[ASTNode]) -> String {
    let mut target_str = String::new();
    for node in raw_ast {
        if let ASTNode::Target(t) = node {
            target_str = t.clone();
        }
    }

    let target_lower = target_str.to_lowercase();
    let target = if target_lower.contains("linux") || target_lower.contains("elf") {
        TargetPlatform::Linux
    } else {
        TargetPlatform::Windows
    };

    let ast = flatten_ast(raw_ast, target);

    let mut asm = String::new();
    let mut user_funcs = Vec::new();
    
    for node in &ast {
        if let ASTNode::Func { name, .. } = node {
            if name != "main" {
                user_funcs.push(name.clone());
            }
        }
    }

    asm.push_str("include 'kernel/FASM/fasmg/extension/format.inc'\n");
    asm.push_str("include 'kernel/FASM/fasmg/extension/wir_ll.inc'\n\n");

    match target {
        TargetPlatform::Windows => {
            asm.push_str("set_build X64, Windows, Console\n\n");

            for node in &ast {
                if let ASTNode::IncludeInc(inc) = node {
                    asm.push_str(&format!("include '{}'\n\n", inc));
                }
            }

            for node in &ast {
                if let ASTNode::Func { name, body } = node {
                    if name != "main" {
                        asm.push_str(&format!("fn_{}:\n", name));
                        push_func_body(&mut asm, body, &user_funcs);
                        asm.push_str("    ret\n\n");
                    }
                }
            }

            asm.push_str("code_block\n");
            asm.push_str("start:\n");
            asm.push_str("    entry start\n");
            for node in &ast {
                if let ASTNode::Func { name, body } = node {
                    if name == "main" {
                        push_func_body(&mut asm, body, &user_funcs);
                    }
                }
            }
            asm.push('\n');

            let mut has_data = false;
            for node in &ast {
                if let ASTNode::Data(content) = node {
                    if !has_data {
                        asm.push_str("data_block\n");
                        has_data = true;
                    }
                    for line in content.lines() {
                        if !line.trim().is_empty() {
                            asm.push_str(&format!("    {}\n", line.trim()));
                        }
                    }
                    asm.push('\n');
                }
            }

            asm.push_str("import_block\n");

            let mut libs: Vec<(String, Vec<String>)> = Vec::new();
            for node in &ast {
                if let ASTNode::Library { name, functions } = node {
                    libs.push((name.clone(), functions.clone()));
                }
            }

            let lib_entries: Vec<String> = libs
                .iter()
                .map(|(lib, _)| format!("{}, '{}'", lib, format!("{}.DLL", lib).to_uppercase()))
                .collect();

            if !lib_entries.is_empty() {
                asm.push_str(&format!("    library {}\n", lib_entries.join(", \\\n        ")));
            }

            for (lib, funcs) in &libs {
                if funcs.is_empty() {
                    continue;
                }
                let func_imports: Vec<String> = funcs.iter().map(|f| format!("{}, '{}'", f, f)).collect();
                asm.push_str(&format!("    import {}, \\\n        {}\n", lib, func_imports.join(", \\\n        ")));
            }
        }
        TargetPlatform::Linux => {
            asm.push_str("set_build X64, Linux, Console\n");
            asm.push_str("entry start\n\n");

            for node in &ast {
                if let ASTNode::IncludeInc(inc) = node {
                    asm.push_str(&format!("include '{}'\n\n", inc));
                }
            }

            let mut has_data = false;
            for node in &ast {
                if let ASTNode::Data(content) = node {
                    if !has_data {
                        asm.push_str("data_block\n");
                        has_data = true;
                    }
                    for line in content.lines() {
                        if !line.trim().is_empty() {
                            asm.push_str(&format!("    {}\n", line.trim()));
                        }
                    }
                    asm.push('\n');
                }
            }

            asm.push_str("code_block\n\n");

            for node in &ast {
                if let ASTNode::Func { name, body } = node {
                    if name != "main" {
                        asm.push_str(&format!("fn_{}:\n", name));
                        push_func_body(&mut asm, body, &user_funcs);
                        asm.push_str("    ret\n\n");
                    }
                }
            }

            asm.push_str("start:\n");
            for node in &ast {
                if let ASTNode::Func { name, body } = node {
                    if name == "main" {
                        push_func_body(&mut asm, body, &user_funcs);
                    }
                }
            }
            asm.push('\n');
        }
    }

    asm
}