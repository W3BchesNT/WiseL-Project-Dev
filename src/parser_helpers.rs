use crate::lexer::Token;
use crate::lexer_helpers::map_token;
use crate::parser::{ASTNode, Expr, Stmt};
use crate::parser_pratt::parse_expr;

pub fn expect(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
    kind: Token,
    description: &str,
) -> Token {
    match iter.next() {
        Some(token) if token == &kind => token.clone(),
        other => panic!(
            "[ERROR.PARSER]: expected {}, got {:?}",
            description, other
        ),
    }
}

pub fn expect_ident(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
    description: &str,
) -> String {
    match iter.next() {
        Some(Token::Identifier(name)) => name.clone(),
        other => panic!(
            "[ERROR.PARSER]: expected {}, got {:?}",
            description, other
        ),
    }
}

pub fn skip_if(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
    kind: Token,
) -> bool {
    if let Some(t) = iter.peek() {
        if t == &&kind {
            iter.next();
            return true;
        }
    }
    false
}

pub fn parse_value(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
) -> Expr {
    match iter.next() {
        Some(Token::FloatNumber(value)) => {
            let val = value.parse::<f32>().unwrap_or(0.0);
            // TODO: parse the negative sign before the number
            Expr::FloatNumber(val)
        }
        Some(Token::WholeNumber(value)) => {
            let val = value.parse::<i32>().unwrap_or(0);
            // TODO: parse the negative sign before the number
            Expr::IntegerNumber(val)
        }
        Some(Token::Identifier(value))       => Expr::Ident(value.clone()),
        Some(Token::StringLiteral(value, _)) => Expr::String(value.clone()),

        other => panic!(
            "[ERROR.PARSER]: expected value (number, identifier, or string), got {:?}",
            other
        ),
    }
}

pub fn handle_let(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
) -> (ASTNode, Stmt) {
    let mutable = skip_if(iter, Token::Mut);
    let name = expect_ident(iter, "variable name");
    skip_if(iter, Token::Colon);
    let ty = expect_ident(iter, "type");
    expect(iter, Token::Assign, "'='");
    let value = parse_expr(iter);
    skip_if(iter, Token::Newline);

    (
        ASTNode::Let { mutable, name: name.clone(), ty: ty.clone(), value: value.clone() },
        Stmt::Let { mutable, name: name.clone(), ty: ty.clone(), value: value.clone() },
    )
}

pub fn handle_raw_statement(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
    token: &Token,
) -> Stmt {
    let mut raw = map_token(token);
    while let Some(t) = iter.peek() {
        if matches!(t, Token::Newline | Token::RBrace) { break }
        raw.push(' ');
        let next_token = iter.next().unwrap();
        raw.push_str(map_token(next_token).as_str());
    }
    skip_if(iter, Token::Newline);

    Stmt::Raw(raw)
}
