use crate::parser::{ASTNode, Expr};
use std::collections::HashMap;

// [CTX] RU: Inline-Функция: тело содержит сырой ASM
// inline [fn_name] () { body }
#[derive(Debug, Clone)]
struct InlineFunc {
    name: String,
    params: Vec<(String, String)>,
    body: Vec<crate::parser::Stmt>,
}

// [CTX] RU: Платформа, ОС
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetPlatform {
    Windows,
    Linux,
}

// [CTX] RU: Локальные переменные
#[derive(Debug, Clone)]
struct LocalVar {
    offset: usize,
    ty: String,
}

// [CTX] RU: Строковой литерал
#[derive(Default)]
struct StringLiterals {
    literals: Vec<(String, String)>,
    counters: HashMap<String, usize>,
}

// [CTX] RU: Интеграция литерала для функции: arg{number}_{type}_fn_{fn_name}
impl StringLiterals {
    fn intern(&mut self, func_name: &str, content: &str) -> String {
        let counter = self.counters.entry(func_name.to_string()).or_insert(0);
        *counter += 1;
        let n = *counter;
        let name = format!("arg{}_{}_fn_{}", n, "str", func_name);
        self.literals.push((name.clone(), content.to_string()));
        name
    }
}

// [CTX] RU: Контекст кодогенерации
struct Ctx<'a> {
    asm: &'a mut String,
    local_vars: HashMap<String, LocalVar>,
    current_var_index: usize,
    stack_size: usize,
    inline_funcs: &'a [InlineFunc],
    user_funcs: &'a [String],
    platform: TargetPlatform,
    label_counter: usize,
    globals: &'a HashMap<String, String>,
    strings: &'a mut StringLiterals,
    epilogue_label: String,
}

impl<'a> Ctx<'a> {
    // [CTX] RU: Свежая метка: .{prefix}_{N}, инкремент счётчика
    fn new_label(&mut self, prefix: &str) -> String {
        let label = format!(".{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    // [CTX] RU: Строка кода с отступом 4 пробела
    fn line(&mut self, s: &str) {
        self.asm.push_str(&format!("    {}\n", s));
    }

    // [CTX] RU: Метка без отступа
    fn label(&mut self, name: &str) {
        self.asm.push_str(&format!("{}:\n", name));
    }

    // [CTX] RU: Сырой текст без отступа
    fn raw(&mut self, s: &str) {
        self.asm.push_str(s);
    }

    // [CTX] RU: Определение типа переменной: локальная, затем глобальная
    fn lookup_var_type(&self, name: &str) -> Option<String> {
        if let Some(lv) = self.local_vars.get(name) {
            return Some(lv.ty.clone());
        }
        if let Some(ty) = self.globals.get(name) {
            return Some(ty.clone());
        }
        None
    }
}

// [COLLECT] RU: Развёртва AST: Раскрытие TargetBlock
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

// [COLLECT] RU: Поиск inline-функции по имени
fn find_inline_func<'a>(name: &str, inline_funcs: &'a [InlineFunc]) -> Option<&'a InlineFunc> {
    inline_funcs.iter().find(|f| f.name == name)
}

// [EXPR] RU: Приведение аргумента вызова к ASM-Операнду с проверкой типа параметра.
fn size_prefix(ty: &str) -> &'static str {
    match ty {
        "i32" | "u32" | "f32" => "dword",
        _ => "qword",
    }
}

// [EXPR] RU: Приведение аргумента вызова к ASM-Операнду с проверкой типа параметра.
fn resolve_arg(ctx: &mut Ctx, arg: &Expr, param_ty: &str, func_name: &str) -> String {

    match (param_ty, arg) {
        ("any", Expr::Ident(name)) => {
            if let Some(lv) = ctx.local_vars.get(name) {
                format!("{} [rbp - {}]", size_prefix(&lv.ty), lv.offset + 8)
            } else {
                name.clone()
            }
        }
        ("any", Expr::String(s)) => format!("\"{}\"", s),
        ("any", Expr::IntegerNumber(n)) => n.to_string(),
        ("any", other) => panic!("[ERROR.CODEGEN]: unsupported any arg: {:?}", other),

        ("str", Expr::Ident(name)) => {
            match ctx.lookup_var_type(name) {
                Some(ty) if ty == "str" => {
                    if let Some(lv) = ctx.local_vars.get(name) {
                        format!("[rbp - {}]", lv.offset + 8)
                    } else {
                        name.clone()
                    }
                }
                Some(ty) => panic!(
                    "[ERROR.CODEGEN]: variable `{}` has type `{}`, expected `str`",
                    name, ty
                ),
                None => panic!(
                    "[ERROR.CODEGEN]: unknown variable `{}`",
                    name
                ),
            }
        }
        ("str", Expr::String(s)) => ctx.strings.intern(func_name, s),
        ("str", other) => panic!(
            "[ERROR.CODEGEN]: parameter of type `str` expects a string literal or str variable, got {:?}",
            other
        ),

        ("i32", Expr::IntegerNumber(n)) => n.to_string(),
        ("i32", Expr::Ident(name)) => {
            match ctx.lookup_var_type(name) {
                Some(ty) if ty == "i32" => {
                    if let Some(lv) = ctx.local_vars.get(name) {
                        format!("dword [rbp - {}]", lv.offset + 8)
                    } else {
                        format!("dword [{}]", name)
                    }
                }
                Some(ty) => panic!(
                    "[ERROR.CODEGEN]: variable `{}` has type `{}`, expected `i32`",
                    name, ty
                ),
                None => panic!(
                    "[ERROR.CODEGEN]: unknown variable `{}`",
                    name
                ),
            }
        }
        ("i32", other) => panic!(
            "[ERROR.CODEGEN]: parameter of type `i32` expects an integer, got {:?}",
            other
        ),

        (_, other) => panic!("[ERROR.CODEGEN]: unsupported argument type: {:?}", other),
    }
}

// [EXPR] RU: Загружаем выражение в целевой регистр (rax, rcx, ...).
fn emit_arg_to_reg(ctx: &mut Ctx, target_reg: &str, arg: &Expr) {
    match arg {
        Expr::Binary { op, left, right } => {
            emit_binary(ctx, op, left, right);
            ctx.line(&format!("mov {}, rax", target_reg));
        }
        Expr::FloatNumber(_n) => {}
        Expr::IntegerNumber(n) => ctx.line(&format!("mov {}, {}", target_reg, n)),
        Expr::String(s) => ctx.line(&format!("mov {}, '{}'", target_reg, s)),
        Expr::Call { name, args } => {
            emit_call(ctx, name, args);
            let src = if target_reg.starts_with('e') || target_reg.ends_with('d') {
                "eax"
            } else {
                "rax"
            };
            ctx.line(&format!("mov {}, {}", target_reg, src));
        }
        Expr::Ident(s) => {
            // [EXPR] RU:
                // Локальная: Загрузка из стека
                // Глобальная: Загрузка по метке
            if let Some(lv) = ctx.local_vars.get(s) {
                let offset = lv.offset;
                let reg = if lv.ty == "i32" {
                    match target_reg {
                        "rax" | "eax" => "eax",
                        "rdi" | "edi" => "edi",
                        "rsi" | "esi" => "esi",
                        "rdx" | "edx" => "edx",
                        "rcx" | "ecx" => "ecx",
                        "r8" | "r8d" => "r8d",
                        "r9" | "r9d" => "r9d",
                        _ => target_reg,
                    }
                } else {
                    match target_reg {
                        "eax" | "rax" => "rax",
                        "edi" | "rdi" => "rdi",
                        "esi" | "rsi" => "rsi",
                        "edx" | "rdx" => "rdx",
                        "ecx" | "rcx" => "rcx",
                        "r8d" | "r8" => "r8",
                        "r9d" | "r9" => "r9",
                        _ => target_reg,
                    }
                };
                ctx.line(&format!("mov {}, [rbp - {}]", reg, offset + 8));
            } else {
                let reg_32 = match target_reg {
                    "rax" | "eax" => "eax",
                    "rcx" | "ecx" => "ecx",
                    "rdx" | "edx" => "edx",
                    "rbx" | "ebx" => "ebx",
                    "rsi" | "esi" => "esi",
                    "rdi" | "edi" => "edi",
                    "r8" | "r8d" => "r8d",
                    "r9" | "r9d" => "r9d",
                    _ => target_reg,
                };
                ctx.line(&format!("mov {}, [{}]", reg_32, s));
            }
        }
    }
}

// [EXPR] RU: Генарато бенарных операций:
fn emit_binary(ctx: &mut Ctx, op: &crate::parser::BinaryOp, left: &Expr, right: &Expr) {
    emit_arg_to_reg(ctx, "rax", left);
    emit_arg_to_reg(ctx, "rcx", right);

    match op {
        crate::parser::BinaryOp::Add => ctx.line("add rax, rcx"),
        crate::parser::BinaryOp::Sub => ctx.line("sub rax, rcx"),
        crate::parser::BinaryOp::Mul => ctx.line("imul rax, rcx"),
        crate::parser::BinaryOp::Div => {
            ctx.line("xor rdx, rdx");
            ctx.line("idiv rcx");
        }
        crate::parser::BinaryOp::Mod => {
            ctx.line("xor rdx, rdx");
            ctx.line("idiv rcx");
            ctx.line("mov rax, rdx");
        }
        crate::parser::BinaryOp::Eq => {
            ctx.line("cmp eax, ecx");
            ctx.line("sete al");
            ctx.line("movzx eax, al");
        }
        crate::parser::BinaryOp::Ne => {
            ctx.line("cmp rax, rcx");
            ctx.line("setne al");
            ctx.line("movzx rax, al");
        }
        crate::parser::BinaryOp::Lt => {
            ctx.line("cmp rax, rcx");
            ctx.line("setl al");
            ctx.line("movzx rax, al");
        }
        crate::parser::BinaryOp::Gt => {
            ctx.line("cmp rax, rcx");
            ctx.line("setg al");
            ctx.line("movzx rax, al");
        }
        crate::parser::BinaryOp::Le => {
            ctx.line("cmp rax, rcx");
            ctx.line("setle al");
            ctx.line("movzx rax, al");
        }
        crate::parser::BinaryOp::Ge => {
            ctx.line("cmp rax, rcx");
            ctx.line("setge al");
            ctx.line("movzx rax, al");
        }
        crate::parser::BinaryOp::And => ctx.line("and rax, rcx"),
        crate::parser::BinaryOp::Or => ctx.line("or rax, rcx"),
    }
}

// [STMT] RU: Генератор вызовов: inline-функция
fn emit_call(ctx: &mut Ctx, name: &str, call_args: &[Expr]) {
    if let Some(func) = find_inline_func(name, ctx.inline_funcs) {
        for stmt in &func.body {
            if let crate::parser::Stmt::Let { name: _let_name, ty, value, .. } = stmt {
                let target = if ty == "i32" { "eax" } else { "rax" };
                emit_arg_to_reg(ctx, target, value);
            }
        }
        let mut out = String::new();
        for stmt in &func.body {
            if let crate::parser::Stmt::Asm(raw) = stmt {
                out.push_str(raw);
                out.push('\n');
            }
        }
        for ((pname, pty), arg) in func.params.iter().zip(call_args.iter()) {
            let val = resolve_arg(ctx, arg, pty, func.name.as_str());
            let is_local = pty == "str" && matches!(arg, Expr::Ident(_));
            if is_local {
                if let Expr::Ident(name) = arg {
                    if let Some(lv) = ctx.local_vars.get(name) {
                        let addr_offset = lv.offset + 8;
                        let len_offset = lv.offset + 16;
                        out = out.replace(
                            &format!("lea rsi, [{}]", val),
                            &format!("mov rsi, [rbp - {}]", addr_offset),
                        );
                        out = out.replace(
                            &format!("lea rdx, [{}]", val),
                            &format!("mov rdx, [rbp - {}]", addr_offset),
                        );
                        out = out.replace(
                            &format!("{{{}}}_len", pname),
                            &format!("[rbp - {}]", len_offset),
                        );
                    }
                }
            }
            out = out.replace(&format!("{{{}}}", pname), &val);
        }
        for line in out.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                ctx.line(trimmed);
            }
        }
        return;
    }

    // [STMT] RU: Регистры ABI для Windows, Linux
    let arg_regs = match ctx.platform {
        TargetPlatform::Linux => ["rdi", "rsi", "rdx", "rcx", "r8", "r9"],
        TargetPlatform::Windows => ["rcx", "rdx", "r8", "r9", "rax", "rax"],
    };

    for (i, a) in call_args.iter().enumerate() {
        if i >= 6 || (ctx.platform == TargetPlatform::Windows && i >= 4) {
            break;
        }
        emit_arg_to_reg(ctx, arg_regs[i], a);
    }

    if ctx.user_funcs.iter().any(|f| f == name) {
        ctx.line(&format!("call fn_{}", name));
    } else {
        ctx.line(&format!("call {}", name));
    }
}

// [STMT] RU: Генерация локальных let
fn emit_let(ctx: &mut Ctx, name: &str, ty: &str, value: &Expr) {
    if ctx.local_vars.contains_key(name) {
        let offset = ctx.local_vars.get(name).unwrap().offset;
        if ty == "str" {
            if let Expr::String(s) = value {
                let lit = ctx.strings.intern("main", s);
                ctx.line(&format!("lea rax, [{}]", lit));
            } else {
                emit_arg_to_reg(ctx, "rax", value);
            }
            ctx.line(&format!("mov [rbp - {}], rax", offset + 8));
            ctx.line(&format!("mov [rbp - {}], rdx", offset + 16));
        } else {
            let target = if ty == "i32" { "eax" } else { "rax" };
            emit_arg_to_reg(ctx, target, value);
            ctx.line(&format!("mov [rbp - {}], rax", offset + 8));
        }
        return;
    }
    let offset = ctx.current_var_index * 8;
    if ty == "str" {
        ctx.current_var_index += 2;
    } else {
        ctx.current_var_index += 1;
    }
    if ty == "str" {
        match value {
            Expr::String(s) => {
                let lit = ctx.strings.intern("main", s);
                ctx.line(&format!("lea rax, [{}]", lit));
            }
            _ => emit_arg_to_reg(ctx, "rax", value),
        }
    } else {
        let target = if ty == "i32" { "eax" } else { "rax" };
        emit_arg_to_reg(ctx, target, value);
    }
    ctx.local_vars.insert(name.to_string(), LocalVar {
        offset,
        ty: ty.to_string(),
    });
    ctx.line(&format!("mov [rbp - {}], rax", offset + 8));
    if ty == "str" {
        ctx.line(&format!("mov [rbp - {}], rdx", offset + 16));
    }
}

// [STMT] RU: Генератор return
fn emit_return(ctx: &mut Ctx, value: Option<&Expr>) {
    if let Some(v) = value {
        emit_arg_to_reg(ctx, "rax", v);
    }
    ctx.line(&format!("jmp {}", ctx.epilogue_label));
}

// [CONTROL] RU: Генератор if/else
fn emit_if(
    ctx: &mut Ctx,
    condition: &Expr,
    then_body: &[crate::parser::Stmt],
    else_body: &Option<Vec<crate::parser::Stmt>>,
) {
    let else_label = ctx.new_label("else");
    let end_label = ctx.new_label("end");

    match condition {
        Expr::Binary { op, left, right } => emit_binary(ctx, op, left, right),
        _ => emit_arg_to_reg(ctx, "rax", condition),
    }
    ctx.line("cmp rax, 0");

    if else_body.is_some() {
        ctx.line(&format!("je {}", else_label));
    } else {
        ctx.line(&format!("je {}", end_label));
    }

    for stmt in then_body {
        emit_stmt(ctx, stmt);
    }

    if let Some(else_stmts) = else_body {
        ctx.line(&format!("jmp {}", end_label));
        ctx.label(&else_label);
        for stmt in else_stmts {
            emit_stmt(ctx, stmt);
        }
    }

    ctx.label(&end_label);
}

// [CONTROL] RU: Генератор циклов while
fn emit_while(ctx: &mut Ctx, condition: &Expr, body: &[crate::parser::Stmt]) {
    let start_label = ctx.new_label("while_start");
    let end_label = ctx.new_label("while_end");

    ctx.label(&start_label);

    match condition {
        Expr::Binary { op, left, right } => emit_binary(ctx, op, left, right),
        _ => emit_arg_to_reg(ctx, "rax", condition),
    }
    ctx.line("cmp rax, 0");
    ctx.line(&format!("je {}", end_label));

    for stmt in body {
        emit_stmt(ctx, stmt);
    }

    ctx.line(&format!("jmp {}", start_label));
    ctx.label(&end_label);
}

// [SRC_MAP] RU: Восстановление исходного кода из Expr для комментариев
fn expr_src(e: &crate::parser::Expr) -> String {
    use crate::parser::Expr;
    match e {
        Expr::Ident(s) => s.clone(),
        Expr::String(s) => format!("\"{}\"", s),
        Expr::IntegerNumber(n) => n.to_string(),
        Expr::FloatNumber(n) => n.to_string(),
        Expr::Binary { op, left, right } => {
            use crate::parser::BinaryOp;
            let o = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Mod => "%",
                BinaryOp::Eq => "==",
                BinaryOp::Ne => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Gt => ">",
                BinaryOp::Le => "<=",
                BinaryOp::Ge => ">=",
                BinaryOp::And => "&&",
                BinaryOp::Or => "||",
            };
            format!("{} {} {}", expr_src(left), o, expr_src(right))
        }
        Expr::Call { name, args } => {
            let a: Vec<String> = args.iter().map(expr_src).collect();
            format!("{}({})", name, a.join(", "))
        }
    }
}

// [SRC_MAP] RU: Восстановление исходного кода из Stmt для комментариев
fn stmt_src(stmt: &crate::parser::Stmt) -> String {
    use crate::parser::Stmt;
    match stmt {
        Stmt::Call { name, args } => {
            let a: Vec<String> = args.iter().map(expr_src).collect();
            format!("{}({})", name, a.join(", "))
        }
        Stmt::Let { mutable, name, ty, value } => {
            let m = if *mutable { "mut " } else { "" };
            format!("let {}{}: {} = {}", m, name, ty, expr_src(value))
        }
        Stmt::Assign { name, value } => format!("{} = {}", name, expr_src(value)),
        Stmt::Return(Some(v)) => format!("return {}", expr_src(v)),
        Stmt::Return(None) => "return".to_string(),
        Stmt::If { condition, .. } => format!("if ({})", expr_src(condition)),
        Stmt::While { condition, .. } => format!("while ({})", expr_src(condition)),
        Stmt::Break => "break".to_string(),
        _ => return String::new(),
    }
}

// [STMT] RU: Диспетчер операторов
fn emit_stmt(ctx: &mut Ctx, stmt: &crate::parser::Stmt) {
    let s = stmt_src(stmt);
    if !s.is_empty() {
        ctx.asm.push_str(&format!("    ; {}\n", s));
    }
    match stmt {
        crate::parser::Stmt::Call { name, args } => emit_call(ctx, name, args),
        crate::parser::Stmt::Let { name, ty, value, .. } => emit_let(ctx, name, ty, value),
        crate::parser::Stmt::Return(value) => emit_return(ctx, value.as_ref()),
        crate::parser::Stmt::Asm(raw) => {
            for line in raw.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    ctx.line(trimmed);
                }
            }
        }
        crate::parser::Stmt::Raw(raw) => ctx.line(raw),
        crate::parser::Stmt::If { condition, then_body, else_body } => {
            emit_if(ctx, condition, then_body, else_body);
        }
        crate::parser::Stmt::While { condition, body } => {
            emit_while(ctx, condition, body);
        }
        crate::parser::Stmt::Assign { name, value } => {
            if let Some(lv) = ctx.local_vars.get(name) {
                let offset = lv.offset;
                emit_arg_to_reg(ctx, "rax", value);
                ctx.line(&format!("mov [rbp - {}], rax", offset + 8));
            }
        }
        crate::parser::Stmt::Break => {}
        crate::parser::Stmt::TargetBlock { target, stmts } => {
            let t_lower = target.to_lowercase();
            let matches = match ctx.platform {
                TargetPlatform::Windows => t_lower.contains("windows"),
                TargetPlatform::Linux => t_lower.contains("linux"),
            };
            if matches {
                for s in stmts {
                    emit_stmt(ctx, s);
                }
            }
        }
    }
}

// [FUNC] RU: Подсчёт количества локальных переменных для выделения стека
fn count_local_vars(stmts: &[crate::parser::Stmt]) -> usize {
    stmts.iter().filter(|s| matches!(s, crate::parser::Stmt::Let { .. })).count()
}

// [FUNC] RU: Генератор тела функции
fn push_func_body(ctx: &mut Ctx, stmts: &[crate::parser::Stmt], params: &[(String, String)]) {
    ctx.local_vars.clear();
    ctx.current_var_index = 0;
    let total_vars = count_local_vars(stmts) + params.len();
    ctx.stack_size = ((total_vars * 8) + 15) & !15;
    ctx.epilogue_label = ctx.new_label("epilogue");

    ctx.raw("    push rbp\n");
    ctx.raw("    mov rbp, rsp\n");
    if ctx.stack_size > 0 {
        ctx.line(&format!("sub rsp, {}", ctx.stack_size));
    }

    let arg_regs = match ctx.platform {
        TargetPlatform::Linux => ["rdi", "rsi", "rdx", "rcx", "r8", "r9"],
        TargetPlatform::Windows => ["rcx", "rdx", "r8", "r9", "rax", "rax"],
    };
    for (i, (pname, pty)) in params.iter().enumerate() {
        if i >= arg_regs.len() { break; }
        let offset = ctx.current_var_index * 8;
        ctx.current_var_index += 1;
        ctx.local_vars.insert(pname.clone(), LocalVar {
            offset,
            ty: pty.clone(),
        });
        let reg = if pty == "i32" {
            match arg_regs[i] {
                "rdi" => "edi", "rsi" => "esi", "rdx" => "edx", "rcx" => "ecx",
                "r8" => "r8d", "r9" => "r9d", r => r,
            }
        } else {
            arg_regs[i]
        };
        ctx.line(&format!("mov [rbp - {}], {}", offset + 8, reg));
    }

    for stmt in stmts {
        emit_stmt(ctx, stmt);
    }

    let ep = ctx.epilogue_label.clone();
    ctx.label(&ep);
    ctx.raw("    mov rsp, rbp\n");
    ctx.raw("    pop rbp\n");
    ctx.raw("    ret\n");
}

// [FUNC] RU: Генератор функций: main -> entry; пользовательские через fn_{name}
fn emit_all_funcs(ctx: &mut Ctx, ast: &[ASTNode]) {
    for node in ast {
        if let ASTNode::Func { name, body, params, is_inline } = node {
            if *is_inline {
                continue;
            }
            match name.as_str() {
                "main" => {
                    ctx.raw("start:\n");
                    push_func_body(ctx, body, params);
                }
                _ => {
                    ctx.raw(&format!("fn_{}:\n", name));
                    push_func_body(ctx, body, params);
                    ctx.raw("\n");
                }
            }
        }
    }
}

// [DATA] RU: Генератор include директив
fn emit_includes(asm: &mut String, ast: &[ASTNode]) {
    for node in ast {
        if let ASTNode::IncludeInc(inc) = node {
            asm.push_str(&format!("include {}\n\n", inc));
        }
    }
}

// [DATA] RU: Генератор data_block:
fn emit_data(asm: &mut String, ast: &[ASTNode], globals: &mut HashMap<String, String>, strings: &StringLiterals) {
    for node in ast {
        match node {
            // @data {}
            ASTNode::Data(content) => {
                for line in content.lines() {
                    if !line.trim().is_empty() {
                        asm.push_str(&format!("    {}\n", line.trim()));
                    }
                }
                asm.push('\n');
            }
            // let [name]: [ty] = [value]
            ASTNode::Let { name, ty, value, .. } => {
                globals.insert(name.clone(), ty.clone());
                match (ty.as_str(), value) {
                    ("string" | "str", Expr::String(val)) => {
                        asm.push_str(&format!("    WSTR {}, '{}'\n", name, val));
                        asm.push_str(&format!("    WSTR_LEN {}\n", name));
                    }
                    ("i64" | "i32", Expr::IntegerNumber(n)) => {
                        asm.push_str(&format!("    WVAR {}, {}, {}\n", name, ty, n));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // [GEN] RU: Генерация строго литерала func_name("hello") -> 
    // WSTR arg{n}_{type}_fn_{fn_name} + WSTR_LEN {name}
    for (name, content) in &strings.literals {
        asm.push_str(&format!("    WSTR {}, '{}', 0\n", name, content));
        asm.push_str(&format!("    WSTR_LEN {}\n", name));
    }
}

// [DATA] RU: Генератор import_block
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

// [GEN] RU: Точка входа кодогенерации: платформа, развёртка AST, собирает функции/глобалы/литерары, склеивает, вывод, out.asm
pub fn generate(raw_ast: &[ASTNode], target_override: Option<&str>) -> String {
    let target = if let Some(t) = target_override {
        if t.to_lowercase().contains("linux") {
            TargetPlatform::Linux
        } else {
            TargetPlatform::Windows
        }
    } else {
        raw_ast
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
            .unwrap_or({
                #[cfg(target_os = "linux")]
                { TargetPlatform::Linux }
                #[cfg(not(target_os = "linux"))]
                { TargetPlatform::Windows }
            })
    };

    let ast = flatten_ast(raw_ast, target);

    let mut user_funcs: Vec<String> = Vec::new();
    let mut inline_funcs: Vec<InlineFunc> = Vec::new();

    for node in &ast {
        if let ASTNode::Func { name, params, body, is_inline } = node {
            if name != "main" {
                user_funcs.push(name.clone());
                if *is_inline {
                    inline_funcs.push(InlineFunc {
                        name: name.clone(),
                        params: params.clone(),
                        body: body.clone(),
                    });
                }
            }
        }
    }

    let mut globals: HashMap<String, String> = HashMap::new();
    for node in &ast {
        if let ASTNode::Let { name, ty, .. } = node {
            globals.insert(name.clone(), ty.clone());
        }
    }

    let mut strings = StringLiterals::default();
    let mut code = String::new();

    {
        let mut ctx = Ctx {
            asm: &mut code,
            local_vars: HashMap::new(),
            current_var_index: 0,
            stack_size: 0,
            inline_funcs: &inline_funcs,
            user_funcs: &user_funcs,
            platform: target,
            label_counter: 0,
            globals: &globals,
            strings: &mut strings,
            epilogue_label: String::new(),
        };
        emit_all_funcs(&mut ctx, &ast);
    }

    // [GEN] RU: Финальная склейка out.asm
    let mut asm = String::new();
    asm.push_str("include 'kernel/FASM/fasmg/extension/format.inc'\n");
    asm.push_str("include 'kernel/FASM/fasmg/extension/wir_ll.inc'\n\n");

    let platform_name = match target {
        TargetPlatform::Windows => "Windows",
        TargetPlatform::Linux => "Linux",
    };
    asm.push_str(&format!("set_build X64, {}, Console\n\n", platform_name));
    asm.push_str("entry start\n\n");

    emit_includes(&mut asm, &ast);
    asm.push_str("data_block\n");
    emit_data(&mut asm, &ast, &mut globals, &strings);

    asm.push_str("\ncode_block\n");
    asm.push_str(&code);
    asm.push('\n');

    if target == TargetPlatform::Windows {
        emit_imports(&mut asm, &ast);
    }

    asm
}