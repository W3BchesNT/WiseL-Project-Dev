use crate::lexer::{Token, TokenKind};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Expr {
    Number(i32),
    Ident(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ASTNode {
    Format(String),
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
        body: String,
    },
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
        } else {
            content.push_str(&inner.value);
            content.push(' ');
        }
    }

    Some(content.trim().to_string())
}

pub fn parse(tokens: &[Token]) -> Vec<ASTNode> {
    let mut statements = Vec::new();
    let mut iter = tokens.iter().peekable();

    while let Some(token) = iter.next() {
        match token.kind {
            TokenKind::FORMAT => {
                if let Some(t) = iter.next() {
                    if t.kind == TokenKind::STRING {
                        let fmt_val = t.value.trim_matches('"').trim_matches('\'').to_string();
                        statements.push(ASTNode::Format(fmt_val));
                    }
                }
            }
            TokenKind::USELIB => {
                if let Some(t) = iter.next() {
                    if t.kind == TokenKind::STRING {
                        let lib_val = t.value.trim_matches('"').trim_matches('\'').to_string();
                        statements.push(ASTNode::UseLib(lib_val));
                    }
                }
            }

            TokenKind::FUNC => {
                let func_name = match iter.next() {
                    Some(t) if t.kind == TokenKind::IDENT => t.value.clone(),
                    _ => String::new(),
                };

                if let Some(t) = iter.next() {
                    if t.kind == TokenKind::LPAREN {
                        let mut depth = 1;
                        while let Some(inner) = iter.next() {
                            if inner.kind == TokenKind::LPAREN {
                                depth += 1;
                            } else if inner.kind == TokenKind::RPAREN {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(body_content) = parse_braces_block(&mut iter) {
                    statements.push(ASTNode::Func {
                        name: func_name,
                        body: body_content,
                    });
                }
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
                    let lib_name = match iter.next() {
                        Some(t) if t.kind == TokenKind::IDENT => t.value.clone(),
                        _ => String::new(),
                    };
                    
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
                }
            }

            TokenKind::ASM => {
                if let Some(asm_content) = parse_braces_block(&mut iter) {
                    statements.push(ASTNode::Asm(asm_content));
                }
            }
            TokenKind::LET => {
                let mut mutable = false;

                if let Some(next_token) = iter.peek() {
                    if next_token.kind == TokenKind::MUT {
                        mutable = true;
                        iter.next();
                    }
                }

                let name = match iter.next() {
                    Some(t) if t.kind == TokenKind::IDENT => t.value.clone(),
                    other => {
                        eprintln!("[ERROR.PARSER]: expected variable name, got {:?}", other);
                        break;
                    }
                };

                if let Some(t) = iter.peek() {
                    if t.kind == TokenKind::COLON || (t.kind == TokenKind::IDENT && t.value == ":") {
                        iter.next();
                    }
                }

                let ty = match iter.next() {
                    Some(t) if t.kind == TokenKind::IDENT => t.value.clone(),
                    other => {
                        eprintln!("[ERROR] expected type, got {:?}", other);
                        break;
                    }
                };

                match iter.next() {
                    Some(t) if t.kind == TokenKind::ASSIGN => {}
                    other => {
                        eprintln!("[ERROR.PARSER]: expected '=', got {:?}", other);
                        break;
                    }
                };

                let value = match iter.next() {
                    Some(t) if t.kind == TokenKind::NUMBER => {
                        let val = t.value.parse::<i32>().unwrap_or(0);
                        Expr::Number(val)
                    }
                    Some(t) if t.kind == TokenKind::IDENT => Expr::Ident(t.value.clone()),
                    other => {
                        eprintln!("[ERROR.PARSER]: expected value, got {:?}", other);
                        break;
                    }
                };

                if let Some(t) = iter.peek() {
                    if t.kind == TokenKind::NEWLINE {
                        iter.next();
                    }
                }

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