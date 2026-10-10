use crate::parser::{ASTNode, Expr};

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
                    TargetPlatform::Windows => t_lower.contains("windows"),
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

fn is_inline_func(name: &str, inline_funcs: &[(String, Vec<(String, String)>, Vec<crate::parser::Stmt>)]) -> bool {
    inline_funcs.iter().any(|(n, _, _)| n == name)
}

fn find_inline_func<'a>(name: &str, inline_funcs: &'a [(String, Vec<(String, String)>, Vec<crate::parser::Stmt>)]) -> Option<&'a (String, Vec<(String, String)>, Vec<crate::parser::Stmt>)> {
    inline_funcs.iter().find(|(n, _, _)| n == name)
}

fn emit_arg_to_reg(asm: &mut String, reg: &str, arg: &Expr) {
    match arg {
        Expr::Number(n) => asm.push_str(&format!("    mov {}, {}\n", reg, n)),
        Expr::Ident(s) => asm.push_str(&format!("    lea {}, [{}]\n", reg, s)),
        Expr::String(s) => asm.push_str(&format!("    mov {}, '{}'\n", reg, s)),
    }
}

fn normalize_braces(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_brace = false;
    for ch in s.chars() {
        match ch {
            '{' => {
                in_brace = true;
                result.push(ch);
            }
            '}' => {
                in_brace = false;
                result.push(ch);
            }
            ' ' if in_brace => {
            }
            _ => result.push(ch),
        }
    }
    result
}

fn expr_to_asm_val(arg: &Expr) -> String {
    match arg {
        Expr::Number(n) => n.to_string(),
        Expr::Ident(id) => id.clone(),
        Expr::String(s) => format!("'{}'", s),
    }
}

// Walk the AST and emit FASM code
fn push_func_body(
    asm: &mut String,
    stmts: &[crate::parser::Stmt],
    _params: &[(String, String)],
    _args: &[Expr],
    inline_funcs: &[(String, Vec<(String, String)>, Vec<crate::parser::Stmt>)],
    user_funcs: &[String],
) {
    for stmt in stmts {
        match stmt {
            crate::parser::Stmt::Call { name, args: call_args } => {
                // Check if this is an inline function
                if let Some((_, fparams, fbody)) = find_inline_func(name, inline_funcs) {
                    for stmt in fbody {
                        if let crate::parser::Stmt::Asm(raw) = stmt {
                            let mut out = normalize_braces(raw);
                            
                            for ((pname, _), arg) in fparams.iter().zip(call_args.iter()) {
                                let val = expr_to_asm_val(arg);
                                out = out.replace(&format!("{{{}}}", pname), &val);
                            }
                            
                            for line in out.lines() {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    asm.push_str(&format!("    {}\n", trimmed));
                                }
                            }
                        }
                    }
                    continue;
                }

                // Regular function call
                let arg_regs = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
                for (i, a) in call_args.iter().enumerate() {
                    if i >= arg_regs.len() {
                        break;
                    }
                    emit_arg_to_reg(asm, arg_regs[i], a);
                }

                if user_funcs.contains(name) {
                    asm.push_str(&format!("    call fn_{}\n", name));
                } else {
                    asm.push_str(&format!("    call {}\n", name));
                }
            }
            crate::parser::Stmt::Let { name, ty: _, value, .. } => {
                emit_arg_to_reg(asm, "rax", value);
                asm.push_str(&format!("    mov [{}], rax\n", name));
            }
            crate::parser::Stmt::Return(value) => {
                if let Some(v) = value {
                    emit_arg_to_reg(asm, "rax", v);
                }
                asm.push_str("    ret\n");
            }
            crate::parser::Stmt::Asm(raw) => {
                // Indent EVERY line of the raw asm block
                for line in raw.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        asm.push_str(&format!("    {}\n", trimmed));
                    }
                }
            }
            crate::parser::Stmt::Raw(raw) => {
                asm.push_str(&format!("    {}\n", raw));
            }
        }
    }
}

fn emit_all_funcs(asm: &mut String, ast: &[ASTNode], inline_funcs: &[(String, Vec<(String, String)>, Vec<crate::parser::Stmt>)], user_funcs: &[String]) {
    for node in ast {
        if let ASTNode::Func { name, body, params } = node {
            emit_func(asm, name, body, params, inline_funcs, user_funcs);
        }
    }
}

fn emit_includes(asm: &mut String, ast: &[ASTNode]) {
    for node in ast {
        if let ASTNode::IncludeInc(inc) = node {
            asm.push_str(&format!("include '{}'\n\n", inc));
        }
    }
}

fn emit_data(asm: &mut String, ast: &[ASTNode]) {
    let mut has_data = false;
    for node in ast {
        match node {
            ASTNode::Data(content) => {
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
            ASTNode::Let { name, ty, value, .. } if ty == "string" || ty == "str" => {
                if let Expr::String(val) = value {
                    if !has_data {
                        asm.push_str("data_block\n");
                        has_data = true;
                    }
                    asm.push_str(&format!("    WSTR {} , '{}' , 13 , 10\n", name, val));
                    asm.push_str(&format!("    WSTR_LEN {}\n", name));
                }
            }
            _ => {}
        }
    }
    if has_data {
        asm.push('\n');
    }
}

// Emit function with proper signature
fn emit_func(asm: &mut String, name: &str, body: &[crate::parser::Stmt], params: &[(String, String)], inline_funcs: &[(String, Vec<(String, String)>, Vec<crate::parser::Stmt>)], user_funcs: &[String]) {
    match name {
        "main" => push_func_body(asm, body, params, &[], inline_funcs, user_funcs),
        _ if !is_inline_func(name, inline_funcs) => {
            asm.push_str(&format!("fn_{}:\n", name));
            push_func_body(asm, body, params, &[], inline_funcs, user_funcs);
            asm.push_str("    ret\n\n");
        }
        _ => {}
    }
}

pub fn generate(raw_ast: &[ASTNode]) -> String {
    let target = raw_ast
        .iter()
        .find_map(|node| {
            if let ASTNode::Target(t) = node {
                Some(if t.to_lowercase().contains("linux") {
                    TargetPlatform::Linux
                } else {
                    TargetPlatform::Windows
                })
            } else {
                None
            }
        })
        .unwrap_or(TargetPlatform::Windows);

    let ast = flatten_ast(raw_ast, target);

    let mut asm = String::new();

    // Collect user functions and inline functions
    let mut user_funcs: Vec<String> = Vec::new();
    let mut inline_funcs: Vec<(String, Vec<(String, String)>, Vec<crate::parser::Stmt>)> = Vec::new();

    for node in &ast {
        if let ASTNode::Func { name, params, body } = node {
            if name != "main" {
                user_funcs.push(name.clone());
                // Check if the function body contains an Asm statement
                let has_asm = body.iter().any(|stmt| matches!(stmt, crate::parser::Stmt::Asm(_)));
                if has_asm {
                    inline_funcs.push((name.clone(), params.clone(), body.clone()));
                }
            }
        }
    }

    asm.push_str("include 'kernel/FASM/fasmg/extension/format.inc'\n");
    asm.push_str("include 'kernel/FASM/fasmg/extension/wir_ll.inc'\n\n");

    match target {
        TargetPlatform::Windows => {
            asm.push_str("set_build X64, Windows, Console\n\n");

            emit_includes(&mut asm, &ast);

            emit_all_funcs(&mut asm, &ast, &inline_funcs, &user_funcs);

            asm.push_str("code_block\n");
            asm.push_str("start:\n");
            asm.push_str("    entry start\n");
            emit_all_funcs(&mut asm, &ast, &inline_funcs, &user_funcs);
            asm.push('\n');

            emit_data(&mut asm, &ast);

            asm.push_str("import_block\n");

            let libs: Vec<_> = ast.iter().filter_map(|node| {
                if let ASTNode::Library { name, functions } = node {
                    Some((name.clone(), functions.clone()))
                } else {
                    None
                }
            }).collect();

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
                let func_imports: Vec<String> =
                    funcs.iter().map(|f| format!("{}, '{}'", f, f)).collect();
                asm.push_str(&format!(
                    "    import {}, \\\n        {}\n",
                    lib,
                    func_imports.join(", \\\n        ")
                ));
            }
        }
        TargetPlatform::Linux => {
            asm.push_str("set_build X64, Linux, Console\n");
            asm.push_str("entry start\n\n");

            emit_includes(&mut asm, &ast);

            emit_data(&mut asm, &ast);

            asm.push_str("code_block\n\n");
            asm.push_str("start:\n");
            emit_all_funcs(&mut asm, &ast, &inline_funcs, &user_funcs);
            asm.push('\n');
        }
    }

    asm
}