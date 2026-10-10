use crate::lexer::{Token, TokenKind};
use crate::parser_helpers::{expect, expect_ident, skip_if, parse_value};

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
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Let {
        mutable: bool,
        name: String,
        ty: String,
        value: Expr,
    },
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
    Let {
        mutable: bool,
        name: String,
        ty: String,
        value: Expr,
    },
    Data(String),
    Library {
        name: String,
        functions: Vec<String>,
    },
    IncludeInc(String),
    Func {
        name: String,
        params: Vec<(String, String)>,
        body: Vec<Stmt>,
    },
    TargetBlock {
        target: String,
        nodes: Vec<ASTNode>,
    }
}

fn parse_braces_block(iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>) -> Option<String> {
    let next_t = iter.peek()?;
    if next_t.kind != TokenKind::LBRACE {
        return None;
    }
    iter.next();

    let mut content = String::new();
    let mut depth = 1;

    while let Some(inner) = iter.next() {
        if inner.kind == TokenKind::LBRACE {
            depth += 1;
            content.push_str(&inner.value);
        } else if inner.kind == TokenKind::RBRACE {
            depth -= 1;
            if depth == 0 {
                break;
            }
            content.push_str(&inner.value);
        } else if inner.kind == TokenKind::NEWLINE {
            content.push('\n');
        } else if inner.kind == TokenKind::LPAREN {
            content.push('(');
        } else if inner.kind == TokenKind::RPAREN {
            content.push(')');
        } else if inner.kind == TokenKind::COMMA {
            content.push(',');
        } else {
            content.push_str(&inner.value);
            content.push(' ');
        }
    }

    Some(content.trim().to_string())
}

// Parse function body into Vec<Stmt>
fn parse_stmts(iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    
    // Skip opening brace
    if let Some(t) = iter.peek() {
        if t.kind == TokenKind::LBRACE {
            iter.next();
        }
    }
    
    let mut depth = 1;
    while let Some(token) = iter.next() {
        if token.kind == TokenKind::RBRACE {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        
        if token.kind == TokenKind::LBRACE {
            depth += 1;
        }
        
        match token.kind {
            TokenKind::LET => {
                let mutable = skip_if(iter, TokenKind::MUT);
                let name = expect_ident(iter, "variable name");
                skip_if(iter, TokenKind::COLON);
                let ty = expect_ident(iter, "type");
                expect(iter, TokenKind::ASSIGN, "'='");
                let value = parse_value(iter);
                skip_if(iter, TokenKind::NEWLINE);
                
                stmts.push(Stmt::Let { mutable, name, ty, value });
            }
            TokenKind::RETURN => {
                // Optional return value
                let value = if let Some(t) = iter.peek() {
                    if t.kind == TokenKind::NEWLINE || t.kind == TokenKind::RBRACE {
                        None
                    } else {
                        Some(parse_value(iter))
                    }
                } else {
                    None
                };
                skip_if(iter, TokenKind::NEWLINE);
                stmts.push(Stmt::Return(value));
            }
            TokenKind::ASM => {
                if let Some(asm_content) = parse_braces_block(iter) {
                    stmts.push(Stmt::Asm(asm_content));
                }
            }
            TokenKind::IDENT => {
                // Try to parse as function call
                if let Some(t) = iter.peek() {
                    if t.kind == TokenKind::LPAREN {
                        let name = token.value.clone();
                        iter.next(); // skip '('
                        let mut args = Vec::new();
                        
                        // Parse arguments
                        while let Some(arg_token) = iter.peek() {
                            if arg_token.kind == TokenKind::RPAREN {
                                iter.next();
                                break;
                            }
                            if arg_token.kind == TokenKind::COMMA {
                                iter.next();
                                continue;
                            }
                            args.push(parse_value(iter));
                        }
                        
                        skip_if(iter, TokenKind::NEWLINE);
                        stmts.push(Stmt::Call { name, args });
                    } else {
                        // Not a call — save as Raw
                        let mut raw = token.value.clone();
                        while let Some(t) = iter.peek() {
                            if t.kind == TokenKind::NEWLINE || t.kind == TokenKind::RBRACE {
                                break;
                            }
                            raw.push(' ');
                            raw.push_str(&iter.next().unwrap().value);
                        }
                        skip_if(iter, TokenKind::NEWLINE);
                        stmts.push(Stmt::Raw(raw));
                    }
                } else {
                    stmts.push(Stmt::Raw(token.value.clone()));
                }
            }
            TokenKind::NEWLINE => {}
            _ => {
                // Everything else — Raw
                let mut raw = token.value.clone();
                while let Some(t) = iter.peek() {
                    if t.kind == TokenKind::NEWLINE || t.kind == TokenKind::RBRACE {
                        break;
                    }
                    raw.push(' ');
                    raw.push_str(&iter.next().unwrap().value);
                }
                skip_if(iter, TokenKind::NEWLINE);
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
        match token.kind {
            TokenKind::IDENT if token.value == "Target" => {
                let is_string = iter.peek().map(|t| t.kind == TokenKind::STRING).unwrap_or(false);
                if is_string {
                    let target_val = iter.next().unwrap().value.trim_matches('"').trim_matches('\'').to_string();
                    statements.push(ASTNode::Target(target_val));
                }
            }
            TokenKind::USELIB => {
                let is_string = iter.peek().map(|t| t.kind == TokenKind::STRING).unwrap_or(false);
                if is_string {
                    let lib_path = iter.next().unwrap().value.trim_matches('"').trim_matches('\'').to_string();
                    
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

            TokenKind::FUNC => {
                let func_name = expect_ident(&mut iter, "function name after 'func'");

                let mut params: Vec<(String, String)> = Vec::new();

                if let Some(t) = iter.peek() {
                    if t.kind == TokenKind::LPAREN {
                        iter.next();
                        let mut depth = 1;
                        let mut cur_name: Option<String> = None;
                        let mut cur_type: Option<String> = None;
                        while let Some(inner) = iter.next() {
                            match inner.kind {
                                TokenKind::LPAREN => depth += 1,
                                TokenKind::RPAREN => {
                                    depth -= 1;
                                    if depth == 0 {
                                        if let Some(n) = cur_name.take() {
                                            let t = cur_type.take().unwrap_or_else(|| "any".to_string());
                                            params.push((n, t));
                                        }
                                        break;
                                    }
                                }
                                TokenKind::COMMA => {
                                    if let Some(n) = cur_name.take() {
                                        let t = cur_type.take().unwrap_or_else(|| "any".to_string());
                                        params.push((n, t));
                                    }
                                }
                                TokenKind::COLON => {}
                                TokenKind::IDENT => {
                                    if cur_name.is_none() {
                                        cur_name = Some(inner.value.clone());
                                    } else if cur_type.is_none() {
                                        cur_type = Some(inner.value.clone());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Parse body as Vec<Stmt> instead of String
                let body = parse_stmts(&mut iter);
                statements.push(ASTNode::Func {
                    name: func_name,
                    params,
                    body,
                });
            }

            TokenKind::DIRECTIVE => {
                if token.value == "@data" {
                    if let Some(inner_content) = parse_braces_block(&mut iter) {
                        let trimmer = inner_content.trim();
                        let cleaned = if trimmer.starts_with("asm") {
                            let without_asm = &trimmer[3..].trim();
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
                } else if token.value == "@library" {
                    let lib_name = expect_ident(&mut iter, "library name");
                    
                    if let Some(block) = parse_braces_block(&mut iter) {
                        let functions: Vec<String> = block
                            .split_whitespace()
                            .map(|s| s.to_string())
                            .filter(|s| !s.is_empty())
                            .collect();

                        statements.push(ASTNode::Library {
                            name: lib_name,
                            functions,
                        });
                    }
                } else if token.value == "@include.inc" {
                    if let Some(content) = parse_braces_block(&mut iter) {
                        let inc_val = content.trim_matches('"').trim_matches('\'').to_string();
                        statements.push(ASTNode::IncludeInc(inc_val));
                    }
                } else if token.value == "@target" {
                    skip_if(&mut iter, TokenKind::ASSIGN);
                    let target_name = match iter.next() {
                        Some(t) if t.kind == TokenKind::STRING => {
                            t.value.trim_matches('"').trim_matches('\'').to_string()
                        }
                        Some(t) if t.kind == TokenKind::IDENT => t.value.clone(),
                        _ => String::new(),
                    };
                    if let Some(block_content) = parse_braces_block(&mut iter) {
                        let sub_tokens = crate::lexer::tokenize(&block_content);
                        let sub_nodes = parse(&sub_tokens);
                        statements.push(ASTNode::TargetBlock{
                            target: target_name,
                            nodes: sub_nodes,
                        });
                    }
                }
            }

            TokenKind::ASM => {
                if let Some(asm_content) = parse_braces_block(&mut iter) {
                    statements.push(ASTNode::Asm(asm_content));
                }
            }
            TokenKind::LET => {
                let mutable = skip_if(&mut iter, TokenKind::MUT);
                let name = expect_ident(&mut iter, "variable name");
                skip_if(&mut iter, TokenKind::COLON);
                let ty = expect_ident(&mut iter, "type");
                expect(&mut iter, TokenKind::ASSIGN, "'='");
                let value = parse_value(&mut iter);
                skip_if(&mut iter, TokenKind::NEWLINE);

                statements.push(ASTNode::Let {
                    mutable,
                    name,
                    ty,
                    value,
                });
            }
            TokenKind::END => {
                break;
            }
            _ => {}
        }
    }

    statements
}