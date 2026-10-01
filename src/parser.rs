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
    }
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

            TokenKind::DIRECTIVE => {
                if token.value == "@data" {
                    if let Some(next_t) = iter.peek() {
                        if next_t.kind == TokenKind::LBRACE {
                            iter.next(); // skip {
                            let mut inner_content = String::new();
                            let mut depth = 1;

                            while let Some(inner) = iter.next() {
                                if inner.kind == TokenKind::LBRACE {
                                    depth += 1;
                                    inner_content.push_str(&inner.value)
                                } else if inner.kind == TokenKind::RBRACE {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                    inner_content.push_str(&inner.value);
                                } else {
                                    inner_content.push_str(&inner.value);
                                    inner_content.push(' ');
                                }
                            }
                            // if in @data -> asm {...}
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
                    }
                } else if token.value == "@library" {
                    // get library name (win)
                    let lib_name = match iter.next() {
                        Some(t) if t.kind == TokenKind::IDENT => t.value.clone(),
                        _ => String::new(),
                    };
                    if let Some(t) = iter.next() {
                        if t.kind == TokenKind::LBRACE {
                            let mut functions = Vec::new();

                            while let Some(inner) = iter.next() {
                                if inner.kind == TokenKind::RBRACE {
                                    break;
                                }
                                if inner.kind == TokenKind::IDENT {
                                    functions.push(inner.value.clone());
                                }
                            }
                            statements.push(ASTNode::Library {
                                name: lib_name,
                                functions,
                            })
                        }
                    }
                }
            }

            TokenKind::ASM => {
                if let Some(next_t) = iter.peek() {
                    if next_t.kind == TokenKind::LBRACE {
                        iter.next();
                        let mut asm_content = String::new();
                        let mut depth = 1;

                        while let Some(inner_token) = iter.next() {
                            if inner_token.kind == TokenKind::LBRACE {
                                depth += 1;
                                asm_content.push_str(&inner_token.value);
                            } else if inner_token.kind == TokenKind::RBRACE {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                asm_content.push_str(&inner_token.value);
                            } else {
                                asm_content.push_str(&inner_token.value);
                                asm_content.push(' ');
                            }
                        }
                        statements.push(ASTNode::Asm(asm_content.trim().to_string()));
                    }
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

                // Variable name
                let name = match iter.next() {
                    Some(t) if t.kind == TokenKind::IDENT => t.value.clone(),
                    other => {
                        eprintln!("[ERROR.PARSER]: expected variable name, got {:?}", other);
                        break;
                    }
                };

                // Optionally check for a colon, if present.
                if let Some(t) = iter.peek() {
                    if t.kind == TokenKind::COLON || (t.kind == TokenKind::IDENT && t.value == ":") {
                        iter.next();
                    }
                }

                // Data type (e.g. i32)
                let ty = match iter.next() {
                    Some(t) if t.kind == TokenKind::IDENT => t.value.clone(),
                    other => {
                        eprintln!("[ERROR] expected type, got {:?}", other);
                        break;
                    }
                };

                // Expect assignment operator `=`
                match iter.next() {
                    Some(t) if t.kind == TokenKind::ASSIGN => {}
                    other => {
                        eprintln!("[ERROR.PARSER]: expected '=', got {:?}", other);
                        break;
                    }
                };

                // Value expression
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