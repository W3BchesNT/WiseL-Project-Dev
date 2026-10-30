use crate::parser::{ASTNode, Expr};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct InlineFunc {
    name: String,
    params: Vec<(String, String)>,
    body: Vec<crate::parser::Stmt>,
}

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

fn is_inline_func(name: &str, inline_funcs: &[InlineFunc]) -> bool {
    inline_funcs.iter().any(|f| f.name == name)
}

fn find_inline_func<'a>(name: &str, inline_funcs: &'a [InlineFunc]) -> Option<&'a InlineFunc> {
    inline_funcs.iter().find(|f| f.name == name)
}

fn emit_arg_to_reg(
    asm: &mut String,
    target_reg: &str,
    arg: &Expr,
    local_vars: &HashMap<String, usize>,
) {
    match arg {
        Expr::Binary { op, left, right } => {
            emit_binary(asm, op, left, right, local_vars);
            asm.push_str(&format!("    mov {}, rax\n", target_reg));
        }
        Expr::FloatNumber(_n) => { /* TODO */ }
        Expr::IntegerNumber(n) => asm.push_str(&format!("    mov {}, {}\n", target_reg, n)),
        Expr::String(s) => asm.push_str(&format!("    mov {}, '{}'\n", target_reg, s)),
        Expr::Ident(s) => {
            if let Some(&offset) = local_vars.get(s) {
                let reg_64 = match target_reg {
                    "eax" | "rax" => "rax",
                    "edi" | "rdi" => "rdi",
                    "esi" | "rsi" => "rsi",
                    "edx" | "rdx" => "rdx",
                    "ecx" | "rcx" => "rcx",
                    "r8d" | "r8" => "r8",
                    "r9d" | "r9" => "r9",
                    _ => target_reg,
                };
                asm.push_str(&format!("    mov {}, [rsp + {}]\n", reg_64, offset));
            } else {
                let reg_32 = match target_reg {
                    "rax" | "eax" => "eax",
                    "rcx" | "ecx" => "ecx",
                    "rdx" | "edx" => "edx",
                    "rbx" | "ebx" => "ebx",
                    "rsi" | "esi" => "esi",
                    "rdi" | "edi" => "edi",
                    "r8"  | "r8d" => "r8d",
                    "r9"  | "r9d" => "r9d",
                    _ => target_reg,
                };
                asm.push_str(&format!("    mov {}, [{}]\n", reg_32, s));
            }
        }
    }
}

fn expr_to_asm_val(arg: &Expr) -> String {
    match arg {
        Expr::FloatNumber(_n) => "TODO".to_string(),
        Expr::IntegerNumber(n) => n.to_string(),
        Expr::Ident(id) => id.clone(),
        Expr::String(s) => format!("'{}'", s),
        Expr::Binary { .. } => "TODO".to_string(),
    }
}

fn count_local_vars(stmts: &[crate::parser::Stmt]) -> usize {
    stmts.iter().filter(|s| matches!(s, crate::parser::Stmt::Let { .. })).count()
}

fn emit_call(
    asm: &mut String,
    name: &str,
    call_args: &[Expr],
    local_vars: &HashMap<String, usize>,
    inline_funcs: &[InlineFunc],
    user_funcs: &[String],
    platform: TargetPlatform,
) {
    if let Some(func) = find_inline_func(name, inline_funcs) {
        for stmt in &func.body {
            if let crate::parser::Stmt::Let { name: _let_name, ty, value, .. } = stmt {
                let target = if ty == "i32" { "eax" } else { "rax" };
                emit_arg_to_reg(asm, target, value, local_vars);
            }
        }
        let mut out = String::new();
        for stmt in &func.body {
            if let crate::parser::Stmt::Asm(raw) = stmt {
                out.push_str(raw);
                out.push('\n');
            }
        }
        for ((pname, _), arg) in func.params.iter().zip(call_args.iter()) {
            let val = expr_to_asm_val(arg);
            out = out.replace(&format!("{{{}}}", pname), &val);
        }
        for line in out.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                asm.push_str(&format!("    {}\n", trimmed));
            }
        }
        return;
    }

    let arg_regs = match platform {
        TargetPlatform::Linux => ["rdi", "rsi", "rdx", "rcx", "r8", "r9"],
        TargetPlatform::Windows => ["rcx", "rdx", "r8", "r9", "rax", "rax"],
    };

    for (i, a) in call_args.iter().enumerate() {
        if i >= 6 || (platform == TargetPlatform::Windows && i >= 4) {
            break;
        }
        emit_arg_to_reg(asm, arg_regs[i], a, local_vars);
    }

    if user_funcs.iter().any(|f| f == name) {
        asm.push_str(&format!("    call fn_{}\n", name));
    } else {
        asm.push_str(&format!("    call {}\n", name));
    }
}

fn emit_let(
    asm: &mut String,
    name: &str,
    ty: &str,
    value: &Expr,
    local_vars: &mut HashMap<String, usize>,
    current_var_index: &mut usize,
) {
    let offset = *current_var_index * 8;
    *current_var_index += 1;
    let target = if ty == "i32" { "eax" } else { "rax" };
    emit_arg_to_reg(asm, target, value, local_vars);
    local_vars.insert(name.to_string(), offset);
    asm.push_str(&format!("    mov [rsp + {}], rax\n", offset));
}

fn emit_return(
    asm: &mut String,
    value: Option<&Expr>,
    local_vars: &HashMap<String, usize>,
    stack_size: usize,
) {
    if let Some(v) = value {
        emit_arg_to_reg(asm, "rax", v, local_vars);
    }
    if stack_size > 0 {
        asm.push_str(&format!("    add rsp, {}\n", stack_size));
    }
    asm.push_str("    ret\n");
}

fn emit_binary(
    asm: &mut String,
    op: &crate::parser::BinaryOp,
    left: &Expr,
    right: &Expr,
    local_vars: &HashMap<String, usize>,
) {
    emit_arg_to_reg(asm, "rax", left, local_vars);
    emit_arg_to_reg(asm, "rcx", right, local_vars);

    match op {
        crate::parser::BinaryOp::Add => asm.push_str("    add rax, rcx\n"),
        crate::parser::BinaryOp::Sub => asm.push_str("    sub rax, rcx\n"),
        crate::parser::BinaryOp::Mul => asm.push_str("    imul rax, rcx\n"),
        crate::parser::BinaryOp::Div => {
            asm.push_str("    xor rdx, rdx\n");
            asm.push_str("    idiv rcx\n");
        }
        crate::parser::BinaryOp::Mod => {
            asm.push_str("    xor rdx, rdx\n");
            asm.push_str("    idiv rcx\n");
            asm.push_str("    mov rax, rdx\n");
        }
        crate::parser::BinaryOp::Eq => {
            asm.push_str("    cmp eax, ecx\n");
            asm.push_str("    sete al\n");
            asm.push_str("    movzx eax, al\n");
        }
        crate::parser::BinaryOp::Ne => {
            asm.push_str("    cmp rax, rcx\n");
            asm.push_str("    setne al\n");
            asm.push_str("    movzx rax, al\n");
        }
        crate::parser::BinaryOp::Lt => {
            asm.push_str("    cmp rax, rcx\n");
            asm.push_str("    setl al\n");
            asm.push_str("    movzx rax, al\n");
        }
        crate::parser::BinaryOp::Gt => {
            asm.push_str("    cmp rax, rcx\n");
            asm.push_str("    setg al\n");
            asm.push_str("    movzx rax, al\n");
        }
        crate::parser::BinaryOp::Le => {
            asm.push_str("    cmp rax, rcx\n");
            asm.push_str("    setle al\n");
            asm.push_str("    movzx rax, al\n");
        }
        crate::parser::BinaryOp::Ge => {
            asm.push_str("    cmp rax, rcx\n");
            asm.push_str("    setge al\n");
            asm.push_str("    movzx rax, al\n");
        }
        crate::parser::BinaryOp::And => asm.push_str("    and rax, rcx\n"),
        crate::parser::BinaryOp::Or => asm.push_str("    or rax, rcx\n"),
    }
}

fn emit_if(
    asm: &mut String,
    condition: &Expr,
    then_body: &[crate::parser::Stmt],
    else_body: &Option<Vec<crate::parser::Stmt>>,
    local_vars: &mut HashMap<String, usize>,
    current_var_index: &mut usize,
    stack_size: usize,
    inline_funcs: &[InlineFunc],
    user_funcs: &[String],
    platform: TargetPlatform,
    label_counter: &mut usize,
) {
    let else_label = format!(".else_{}", label_counter);
    let end_label = format!(".end_{}", label_counter);
    *label_counter += 1;

    match condition {
        Expr::Binary { op, left, right } => emit_binary(asm, op, left, right, local_vars),
        _ => emit_arg_to_reg(asm, "rax", condition, local_vars),
    }
    asm.push_str("    cmp rax, 0\n");

    if else_body.is_some() {
        asm.push_str(&format!("    je {}\n", else_label));
    } else {
        asm.push_str(&format!("    je {}\n", end_label));
    }

    for stmt in then_body {
        emit_stmt(asm, stmt, local_vars, current_var_index, stack_size,
                  inline_funcs, user_funcs, platform, label_counter);
    }

    if let Some(else_stmts) = else_body {
        asm.push_str(&format!("    jmp {}\n", end_label));
        asm.push_str(&format!("{}:\n", else_label));
        for stmt in else_stmts {
            emit_stmt(asm, stmt, local_vars, current_var_index, stack_size,
                      inline_funcs, user_funcs, platform, label_counter);
        }
    }

    asm.push_str(&format!("{}:\n", end_label));
}

fn emit_while(
    asm: &mut String,
    condition: &Expr,
    body: &[crate::parser::Stmt],
    local_vars: &mut HashMap<String, usize>,
    current_var_index: &mut usize,
    stack_size: usize,
    inline_funcs: &[InlineFunc],
    user_funcs: &[String],
    platform: TargetPlatform,
    label_counter: &mut usize,
) {
    let start_label = format!(".while_start_{}", label_counter);
    let end_label = format!(".while_end_{}", label_counter);
    *label_counter += 1;

    asm.push_str(&format!("{}:\n", start_label));

    match condition {
        Expr::Binary { op, left, right } => emit_binary(asm, op, left, right, local_vars),
        _ => emit_arg_to_reg(asm, "rax", condition, local_vars),
    }
    asm.push_str("    cmp rax, 0\n");
    asm.push_str(&format!("    je {}\n", end_label));

    for stmt in body {
        emit_stmt(asm, stmt, local_vars, current_var_index, stack_size,
                  inline_funcs, user_funcs, platform, label_counter);
    }

    asm.push_str(&format!("    jmp {}\n", start_label));
    asm.push_str(&format!("{}:\n", end_label));
}

fn emit_stmt(
    asm: &mut String,
    stmt: &crate::parser::Stmt,
    local_vars: &mut HashMap<String, usize>,
    current_var_index: &mut usize,
    stack_size: usize,
    inline_funcs: &[InlineFunc],
    user_funcs: &[String],
    platform: TargetPlatform,
    label_counter: &mut usize,
) {
    match stmt {
        crate::parser::Stmt::Call { name, args } => {
            emit_call(asm, name, args, local_vars, inline_funcs, user_funcs, platform);
        }
        crate::parser::Stmt::Let { name, ty, value, .. } => {
            emit_let(asm, name, ty, value, local_vars, current_var_index);
        }
        crate::parser::Stmt::Return(value) => {
            emit_return(asm, value.as_ref(), local_vars, stack_size);
        }
        crate::parser::Stmt::Asm(raw) => {
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
        crate::parser::Stmt::If { condition, then_body, else_body } => {
            emit_if(
                asm,
                condition,
                then_body,
                else_body,
                local_vars,
                current_var_index,
                stack_size,
                inline_funcs,
                user_funcs,
                platform,
                label_counter,
            );
        }
        crate::parser::Stmt::While { condition, body } => {
            emit_while(
                asm,
                condition,
                body,
                local_vars,
                current_var_index,
                stack_size,
                inline_funcs,
                user_funcs,
                platform,
                label_counter,
            );
        }
        crate::parser::Stmt::Assign { name, value } => {
            if let Some(&offset) = local_vars.get(name) {
                emit_arg_to_reg(asm, "rax", value, local_vars);
                asm.push_str(&format!("    mov [rsp + {}], rax\n", offset));
            }
        }
        crate::parser::Stmt::Break => {
            // TODO: break logic
        }
    }
}

fn push_func_body(
    asm: &mut String,
    stmts: &[crate::parser::Stmt],
    _params: &[(String, String)],
    _args: &[Expr],
    inline_funcs: &[InlineFunc],
    user_funcs: &[String],
    platform: TargetPlatform,
    label_counter: &mut usize,
) {
    let mut local_vars: HashMap<String, usize> = HashMap::new();
    let mut current_var_index = 0;
    let total_vars = count_local_vars(stmts);
    let stack_size = ((total_vars * 8) + 15) & !15;

    if stack_size > 0 {
        asm.push_str(&format!("    sub rsp, {}\n", stack_size));
    }

    for stmt in stmts {
        emit_stmt(
            asm,
            stmt,
            &mut local_vars,
            &mut current_var_index,
            stack_size,
            inline_funcs,
            user_funcs,
            platform,
            label_counter,
        );
    }

    if stmts.last().map_or(true, |last| !matches!(last, crate::parser::Stmt::Return(_))) {
        if stack_size > 0 {
            asm.push_str(&format!("    add rsp, {}\n", stack_size));
        }
    }
}

fn emit_all_funcs(
    asm: &mut String,
    ast: &[ASTNode],
    inline_funcs: &[InlineFunc],
    user_funcs: &[String],
    platform: TargetPlatform,
) {
    let mut label_counter: usize = 0;
    for node in ast {
        if let ASTNode::Func { name, body, params } = node {
            emit_func(asm, name, body, params, inline_funcs, user_funcs, platform, &mut label_counter);
        }
    }
}

fn emit_includes(asm: &mut String, ast: &[ASTNode]) {
    for node in ast {
        if let ASTNode::IncludeInc(inc) = node {
            asm.push_str(&format!("include {}\n\n", inc));
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
            ASTNode::Let { name, ty, value, .. } => {
                if !has_data {
                    asm.push_str("data_block\n");
                    has_data = true;
                }
                match (ty.as_str(), value) {
                    ("string" | "str", Expr::String(val)) => {
                        asm.push_str(&format!("    WSTR {}, '{}', 13, 10\n", name, val));
                        asm.push_str(&format!("    WSTR_LEN {}\n", name));
                    }
                    ("i32", Expr::IntegerNumber(n)) => {
                        asm.push_str(&format!("    {} dd {}\n", name, n));
                    }
                    ("i64", Expr::IntegerNumber(n)) => {
                        asm.push_str(&format!("    {} dq {}\n", name, n));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    if has_data {
        asm.push('\n');
    }
}

fn emit_func(
    asm: &mut String,
    name: &str,
    body: &[crate::parser::Stmt],
    params: &[(String, String)],
    inline_funcs: &[InlineFunc],
    user_funcs: &[String],
    platform: TargetPlatform,
    label_counter: &mut usize,
) {
    match name {
        "main" => push_func_body(asm, body, params, &[], inline_funcs, user_funcs, platform, label_counter),
        _ if !is_inline_func(name, inline_funcs) => {
            asm.push_str(&format!("fn_{}:\n", name));
            push_func_body(asm, body, params, &[], inline_funcs, user_funcs, platform, label_counter);
            asm.push_str("    ret\n\n");
        }
        _ => {}
    }
}

fn emit_imports(asm: &mut String, ast: &[ASTNode]) {
    asm.push_str("import_block\n");
    let mut merged: Vec<(&String, Vec<&String>)> = Vec::new();

    for node in ast {
        if let ASTNode::Library { name, functions } = node {
            if let Some(entry) = merged.iter_mut().find(|(n, _)| *n == name) {
                for f in functions {
                    if !entry.1.iter().any(|e| *e == f) {
                        entry.1.push(f);
                    }
                }
            } else {
                merged.push((name, functions.iter().collect()));
            }
        }
    }

    if !merged.is_empty() {
        let lib_entries: Vec<String> = merged
            .iter()
            .map(|(lib, _)| {
                let dll = format!("{}.DLL", lib).to_uppercase();
                format!("{}, '{}'", lib, dll)
            })
            .collect();
        asm.push_str(&format!("    library {}\n", lib_entries.join(", \\\n        ")));
    }

    for (lib, funcs) in &merged {
        if funcs.is_empty() {
            continue;
        }
        let func_imports: Vec<String> = funcs.iter().map(|f| format!("{}, '{}'", f, f)).collect();
        asm.push_str(&format!(
            "    import {}, \\\n        {}\n",
            lib,
            func_imports.join(", \\\n        ")
        ));
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

    let mut user_funcs: Vec<String> = Vec::new();
    let mut inline_funcs: Vec<InlineFunc> = Vec::new();

    for node in &ast {
        if let ASTNode::Func { name, params, body } = node {
            if name != "main" {
                user_funcs.push(name.clone());
                let has_asm = body.iter().any(|stmt| matches!(stmt, crate::parser::Stmt::Asm(_)));
                if has_asm {
                    inline_funcs.push(InlineFunc {
                        name: name.clone(),
                        params: params.clone(),
                        body: body.clone(),
                    });
                }
            }
        }
    }

    asm.push_str("include 'kernel/FASM/fasmg/extension/format.inc'\n");
    asm.push_str("include 'kernel/FASM/fasmg/extension/wir_ll.inc'\n\n");

    let platform_name = match target {
        TargetPlatform::Windows => "Windows",
        TargetPlatform::Linux => "Linux",
    };
    asm.push_str(&format!("set_build X64, {}, Console\n\n", platform_name));

    asm.push_str("entry start\n\n");

    emit_includes(&mut asm, &ast);
    emit_data(&mut asm, &ast);

    asm.push_str("code_block\n\n");
    asm.push_str("start:\n");
    emit_all_funcs(&mut asm, &ast, &inline_funcs, &user_funcs, target);
    asm.push('\n');

    if target == TargetPlatform::Windows {
        emit_imports(&mut asm, &ast);
    }

    asm
}