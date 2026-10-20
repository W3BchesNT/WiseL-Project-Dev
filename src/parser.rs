use crate::lexer::{Token, tokenize};
use crate::lexer_helpers::{map_token};
use crate::parser_helpers::{
    expect,
    expect_ident,
    skip_if,
    handle_let,
    handle_raw_statement,
};
use crate::parser_pratt::parse_expr;

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Expr {
    FloatNumber(f32),
    IntegerNumber(i32),
    Ident(String),
    String(String),

    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
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
    if **next_t != Token::LBrace {
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
                if depth == 0 { break }
                content.push('}');
            }
            Token::Newline => content.push('\n'),
            Token::LParen => content.push('('),
            Token::RParen => content.push(')'),
            Token::Comma => content.push(','),

            _ => {
                content.push_str(map_token(inner).as_str());
                content.push(' ');
            }
        }
    }
    Some(content.trim().to_string())
}

// Parse function body into Vec<Stmt>
fn parse_stmts(iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>) -> Vec<Stmt> {
    let mut stmts = Vec::new();

    while skip_if(iter, Token::Newline) {}
    expect(iter, Token::LBrace, "'{'");

    let mut depth: usize = 0;
    while let Some(token) = iter.next() {
        match token {
            Token::LBrace => { depth += 1 },
            Token::RBrace => { if depth == 0 { break }; depth -= 1 },
            _             => {},
        };
        match token {
            Token::Let => { let (_, stmt) = handle_let(iter); stmts.push(stmt) },
            Token::Return => {
                // Optional return value
                let value = if let Some(t) = iter.peek() {
                    if matches!(t, Token::Newline | Token::RBrace) { None }
                    else { Some(parse_expr(iter)) }
                } else { None };
                skip_if(iter, Token::Newline);
                stmts.push(Stmt::Return(value));
            }
            Token::ASM(content)                 => { stmts.push(Stmt::Asm(content.to_string())) }
            Token::Identifier(identifier_value) => {
                // Try to parse as function call
                if let Some(t) = iter.peek() {
                    if matches!(t, Token::LParen) {
                        let name = identifier_value.clone();
                        iter.next(); // skip '('
                        let mut args = Vec::new();

                        // Parse arguments
                        while let Some(arg_token) = iter.peek() {
                            match arg_token {
                                Token::RParen => { iter.next(); break },
                                Token::Comma  => { iter.next(); continue },
                                _             => {},
                            }
                            args.push(parse_expr(iter));
                        }

                        skip_if(iter, Token::Newline);
                        stmts.push(Stmt::Call { name, args });
                    } else {
                        // Not a call — save as Raw
                        stmts.push(handle_raw_statement(iter, token));
                    }
                } else {
                    stmts.push(Stmt::Raw(identifier_value.clone()));
                }
            }
            Token::Newline => {}
            _              => {
                // Everything else — Raw
                stmts.push(handle_raw_statement(iter, token));
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
                let target = match iter.next() {
                    Some(Token::StringLiteral(value, _)) => value.clone(),
                    other => panic!(
                        "[ERROR.PARSER]: expected target string after 'Target', got {:?}",
                        other
                    ),
                };

                statements.push(ASTNode::Target(target));
            },

            Token::UseLib => {
                let lib_path = match iter.next() {
                    Some(Token::StringLiteral(value, _)) => value.clone(),
                    other => panic!(
                        "[ERROR.PARSER]: expected library path after 'UseLib', got {:?}",
                        other
                    ),
                };

                match std::fs::read_to_string(&lib_path) {
                    Ok(lib_source) => {
                        let sub_tokens = tokenize(&lib_source);
                        let sub_nodes = parse(&sub_tokens);
                        statements.extend(sub_nodes);
                    }
                    Err(err) => {
                        eprintln!(
                            "[ERROR] Cannot read library '{lib_path}': {err}"
                        );
                    }
                }
            },

            Token::Func => {
                let func_name = expect_ident(&mut iter, "function name after 'func'");
                let mut params: Vec<(String, String)> = Vec::new();

                if let Some(Token::LParen) = iter.peek() {
                    iter.next();
                    let mut depth = 1;
                    let mut cur_name: Option<String> = None;
                    let mut cur_type: Option<String> = None;
                    while let Some(inner) = iter.next() {
                        match inner {
                            Token::LParen => { depth += 1 },
                            Token::RParen => {
                                depth -= 1;
                                if depth == 0 {
                                    if let Some(n) = cur_name.take() {
                                        let t = cur_type.take().unwrap_or_else(|| "any".to_string());
                                        params.push((n, t));
                                    }
                                    break;
                                }
                            },
                            Token::Comma => if let Some(n) = cur_name.take() {
                                let t = cur_type.take().unwrap_or_else(|| "any".to_string());
                                params.push((n, t));
                            },
                            // Token::Colon => {},
                            Token::Identifier(identifier_value) => {
                                if cur_name.is_none() { cur_name = Some(identifier_value.clone()) }
                                else if cur_type.is_none() { cur_type = Some(identifier_value.clone()) }
                            },
                            _ => {},
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
            },

            Token::RawDirective(directive_kind, directive_value) => match directive_kind.as_str() {
                "@data" => {
                    let trimmed = directive_value.trim();
                    let cleaned = if trimmed.starts_with("asm") {
                        let without_asm = &trimmed[3..].trim();
                        if without_asm.starts_with('{') && without_asm.ends_with('}') {
                            &without_asm[1..without_asm.len() - 1]
                        } else { without_asm }
                    } else { trimmed };

                    statements.push(ASTNode::Data(cleaned.trim().to_string()));
                },
                other   => { eprintln!("[ERROR] unknown raw directive {other}") },
            },
            Token::Directive(directive_kind) => {
                match directive_kind.as_str() {
                    "@library" => {
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
                    },
                    "@include.inc" => if let Some(content) = parse_braces_block(&mut iter) {
                        let inc_val = content.to_string();
                        statements.push(ASTNode::IncludeInc(inc_val));
                    },
                    "@target" => {
                        skip_if(&mut iter, Token::Assign);
                        let target_name = match iter.next() {
                            Some(Token::StringLiteral(value, _)) => {
                                value.to_string()
                            }
                            Some(Token::Identifier(value)) => value.clone(),
                            _ => String::new(),
                        };
                        if let Some(block_content) = parse_braces_block(&mut iter) {
                            let sub_tokens = tokenize(&block_content);
                            let sub_nodes = parse(&sub_tokens);
                            statements.push(ASTNode::TargetBlock{
                                target: target_name,
                                nodes: sub_nodes,
                            });
                        }
                    },
                    _ => {}
                }
            },

            Token::ASM(content) => { statements.push(ASTNode::Asm(content.to_string())) },
            Token::Let => { let (ast_node, _) = handle_let(&mut iter); statements.push(ast_node) },
            Token::End => { break },
            _          => {},
        }
    }

    statements
}