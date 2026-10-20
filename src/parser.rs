use crate::lexer::Token;
use crate::parser_helpers::{expect_ident, skip_if, parse_value};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Expr {
    Number(i32),
    Ident(String),
    String(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Stmt {
    Call { name: String, args: Vec<Expr> },
    Let { mutable: bool, name: String, ty: String, value: Expr },
    Return(Option<Expr>),
    Asm(String),
    Raw(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ASTNode {
    Target(String),
    UseLib(String),
    Asm(String),
    Let { mutable: bool, name: String, ty: String, value: Expr },
    Data(String),
    Library { name: String, functions: Vec<String> },
    IncludeInc(String),
    Func { name: String, params: Vec<(String, String)>, body: Vec<Stmt> },
    TargetBlock { target: String, nodes: Vec<ASTNode> },
}

fn token_to_string(t: &Token) -> String {
    match t {
        Token::Identifier(s) => s.clone(),
        Token::Bool(b) => b.to_string(),
        Token::StringLiteral(s) => s.clone(),
        Token::WholeNumber(s) => s.clone(),
        Token::FloatNumber(s) => s.clone(),
        Token::Let => "let".into(),
        Token::Mut => "mut".into(),
        Token::Static => "static".into(),
        Token::Func => "func".into(),
        Token::If => "if".into(),
        Token::Else => "else".into(),
        Token::While => "while".into(),
        Token::Break => "break".into(),
        Token::Return => "return".into(),
        Token::Switch => "switch".into(),
        Token::Case => "case".into(),
        Token::Match => "match".into(),
        Token::Assign => "=".into(),
        Token::Div => "/".into(),
        Token::Mod => "%".into(),
        Token::Dot => ".".into(),
        Token::Arrow => "->".into(),
        Token::Plus => "+".into(),
        Token::Minus => "-".into(),
        Token::Star => "*".into(),
        Token::Exponent => "**".into(),
        Token::PlusPlus => "++".into(),
        Token::MinusMinus => "--".into(),
        Token::Eq => "==".into(),
        Token::InEq => "!=".into(),
        Token::GreaterThan => ">".into(),
        Token::LessThan => "<".into(),
        Token::GreaterThanOrEq => ">=".into(),
        Token::LessThanOrEq => "<=".into(),
        Token::Not => "!".into(),
        Token::PlusAssign => "+=".into(),
        Token::MinusAssign => "-=".into(),
        Token::StarAssign => "*=".into(),
        Token::DivAssign => "/=".into(),
        Token::ExponentAssign => "**=".into(),
        Token::ModAssign => "%=".into(),
        Token::LParen => "(".into(),
        Token::RParen => ")".into(),
        Token::LBrace => "{".into(),
        Token::RBrace => "}".into(),
        Token::LBracket => "[".into(),
        Token::RBracket => "]".into(),
        Token::Colon => ":".into(),
        Token::Comma => ",".into(),
        Token::Newline => "\n".into(),
        Token::Semicolon => ";".into(),
        Token::UseLib => "UseLib".into(),
        Token::ASM(_) => "asm".into(),
        Token::Format => "Format".into(),
        Token::Directive(s) => s.clone(),
        Token::RawDirective(k, _) => k.clone(),
        Token::End => "".into(),
    }
}

fn parse_braces_block(iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>) -> Option<String> {
    if !matches!(iter.peek(), Some(Token::LBrace)) {
        return None;
    }
    iter.next();

    let mut content = String::new();
    let mut depth = 1;

    while let Some(inner) = iter.next() {
        match inner {
            Token::LBrace => {
                depth += 1;
                content.push('{');
            }
            Token::RBrace => {
                depth -= 1;
                if depth == 0 { break; }
                content.push('}');
            }
            Token::Newline => content.push('\n'),
            Token::LParen => content.push('('),
            Token::RParen => content.push(')'),
            Token::Comma => content.push(','),
            Token::ASM(raw) | Token::RawDirective(_, raw) => {
                content.push_str(raw);
                content.push(' ');
            }
            _ => {
                content.push_str(&token_to_string(inner));
                content.push(' ');
            }
        }
    }
    Some(content.trim().to_string())
}

fn parse_stmts(iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>) -> Vec<Stmt> {
    let mut stmts = Vec::new();

    if matches!(iter.peek(), Some(Token::LBrace)) {
        iter.next();
    }

    let mut depth = 1;
    while let Some(token) = iter.next() {
        if matches!(token, Token::RBrace) {
            depth -= 1;
            if depth == 0 { break; }
        }
        if matches!(token, Token::LBrace) {
            depth += 1;
        }

        match token {
            Token::Let => {
                let mutable = skip_if(iter, |t| matches!(t, Token::Mut));
                let name = expect_ident(iter, "variable name");
                skip_if(iter, |t| matches!(t, Token::Colon));
                let ty = expect_ident(iter, "type");
                skip_if(iter, |t| matches!(t, Token::Assign));
                let value = parse_value(iter);
                skip_if(iter, |t| matches!(t, Token::Newline));
                stmts.push(Stmt::Let { mutable, name, ty, value });
            }
            Token::Return => {
                let value = match iter.peek() {
                    Some(Token::Newline) | Some(Token::RBrace) | None => None,
                    _ => Some(parse_value(iter)),
                };
                skip_if(iter, |t| matches!(t, Token::Newline));
                stmts.push(Stmt::Return(value));
            }
            Token::ASM(raw) => {
                stmts.push(Stmt::Asm(raw.clone()));
            }
            Token::Identifier(name) => {
                if matches!(iter.peek(), Some(Token::LParen)) {
                    let name = name.clone();
                    iter.next();
                    let mut args = Vec::new();
                    while let Some(arg_token) = iter.peek() {
                        if matches!(arg_token, Token::RParen) { iter.next(); break; }
                        if matches!(arg_token, Token::Comma) { iter.next(); continue; }
                        args.push(parse_value(iter));
                    }
                    skip_if(iter, |t| matches!(t, Token::Newline));
                    stmts.push(Stmt::Call { name, args });
                } else {
                    let mut raw = name.clone();
                    while let Some(t) = iter.peek() {
                        if matches!(t, Token::Newline) || matches!(t, Token::RBrace) { break; }
                        raw.push(' ');
                        raw.push_str(&token_to_string(iter.next().unwrap()));
                    }
                    skip_if(iter, |t| matches!(t, Token::Newline));
                    stmts.push(Stmt::Raw(raw));
                }
            }
            Token::Newline => {}
            _ => {
                let mut raw = token_to_string(token);
                while let Some(t) = iter.peek() {
                    if matches!(t, Token::Newline) || matches!(t, Token::RBrace) { break; }
                    raw.push(' ');
                    raw.push_str(&token_to_string(iter.next().unwrap()));
                }
                skip_if(iter, |t| matches!(t, Token::Newline));
                stmts.push(Stmt::Raw(raw));
            }
        }
    }

    stmts
}

pub fn parse(tokens: &[Token]) -> Vec<ASTNode> {
    let mut statements = Vec::new();
    let mut iter = tokens.iter().peekable();

    while let Some(token) = iter.next() {
        match token {
            Token::Identifier(name) if name == "Target" => {
                let is_string = matches!(iter.peek(), Some(Token::StringLiteral(_)));
                if is_string {
                    if let Some(Token::StringLiteral(s)) = iter.next() {
                        statements.push(ASTNode::Target(s.clone()));
                    }
                }
            }
            Token::UseLib => {
                let is_string = matches!(iter.peek(), Some(Token::StringLiteral(_)));
                if is_string {
                    if let Some(Token::StringLiteral(lib_path)) = iter.next() {
                        let lib_path = lib_path.clone();
                        match std::fs::read_to_string(&lib_path) {
                            Ok(lib_source) => {
                                let sub_tokens = crate::lexer::tokenize(&lib_source);
                                let sub_nodes = parse(&sub_tokens);
                                statements.extend(sub_nodes);
                            }
                            Err(err) => {
                                eprintln!("[ERROR] Cannot read library '{lib_path}': {err}");
                            }
                        }
                    }
                }
            }
            Token::Func => {
                let func_name = expect_ident(&mut iter, "function name after 'func'");
                let mut params: Vec<(String, String)> = Vec::new();

                if matches!(iter.peek(), Some(Token::LParen)) {
                    iter.next();
                    let mut depth = 1;
                    let mut cur_name: Option<String> = None;
                    let mut cur_type: Option<String> = None;
                    while let Some(inner) = iter.next() {
                        match inner {
                            Token::LParen => depth += 1,
                            Token::RParen => {
                                depth -= 1;
                                if depth == 0 {
                                    if let Some(n) = cur_name.take() {
                                        let t = cur_type.take().unwrap_or_else(|| "any".to_string());
                                        params.push((n, t));
                                    }
                                    break;
                                }
                            }
                            Token::Comma => {
                                if let Some(n) = cur_name.take() {
                                    let t = cur_type.take().unwrap_or_else(|| "any".to_string());
                                    params.push((n, t));
                                }
                            }
                            Token::Colon => {}
                            Token::Identifier(s) => {
                                if cur_name.is_none() {
                                    cur_name = Some(s.clone());
                                } else if cur_type.is_none() {
                                    cur_type = Some(s.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }

                let body = parse_stmts(&mut iter);
                statements.push(ASTNode::Func { name: func_name, params, body });
            }
            Token::Directive(dir) => {
                if dir == "@data" {
                    if let Some(inner_content) = parse_braces_block(&mut iter) {
                        let trimmer = inner_content.trim();
                        let cleaned = if trimmer.starts_with("asm") {
                            let without_asm = trimmer[3..].trim();
                            if without_asm.starts_with('{') && without_asm.ends_with('}') {
                                &without_asm[1..without_asm.len() - 1]
                            } else {
                                without_asm
                            }
                        } else {
                            trimmer
                        };
                        statements.push(ASTNode::Data(cleaned.trim().to_string()));
                    }
                } else if dir == "@library" {
                    let lib_name = expect_ident(&mut iter, "library name");
                    if let Some(block) = parse_braces_block(&mut iter) {
                        let functions: Vec<String> = block
                            .split_whitespace()
                            .map(|s| s.to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        statements.push(ASTNode::Library { name: lib_name, functions });
                    }
                } else if dir == "@include.inc" {
                    if let Some(content) = parse_braces_block(&mut iter) {
                        let inc_val = content.trim_matches('"').trim_matches('\'').to_string();
                        statements.push(ASTNode::IncludeInc(inc_val));
                    }
                } else if dir == "@target" {
                    skip_if(&mut iter, |t| matches!(t, Token::Assign));
                    let target_name = match iter.next() {
                        Some(Token::StringLiteral(s)) => s.clone(),
                        Some(Token::Identifier(s)) => s.clone(),
                        _ => String::new(),
                    };
                    if let Some(block_content) = parse_braces_block(&mut iter) {
                        let sub_tokens = crate::lexer::tokenize(&block_content);
                        let sub_nodes = parse(&sub_tokens);
                        statements.push(ASTNode::TargetBlock {
                            target: target_name,
                            nodes: sub_nodes,
                        });
                    }
                }
            }
            Token::ASM(raw) => {
                statements.push(ASTNode::Asm(raw.clone()));
            }
            Token::Let => {
                let mutable = skip_if(&mut iter, |t| matches!(t, Token::Mut));
                let name = expect_ident(&mut iter, "variable name");
                skip_if(&mut iter, |t| matches!(t, Token::Colon));
                let ty = expect_ident(&mut iter, "type");
                skip_if(&mut iter, |t| matches!(t, Token::Assign));
                let value = parse_value(&mut iter);
                skip_if(&mut iter, |t| matches!(t, Token::Newline));
                statements.push(ASTNode::Let { mutable, name, ty, value });
            }
            Token::End => break,
            _ => {}
        }
    }

    statements
}