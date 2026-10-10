use crate::lexer::{Token, TokenKind};
use crate::parser::Expr;

pub fn expect(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
    kind: TokenKind,
    description: &str,
) -> Token {
    match iter.next() {
        Some(t) if t.kind == kind => t.clone(),
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
    expect(iter, TokenKind::IDENT, description).value
}

pub fn skip_if(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
    kind: TokenKind,
) -> bool {
    if let Some(t) = iter.peek() {
        if t.kind == kind {
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
        Some(t) if t.kind == TokenKind::NUMBER => {
            let val = t.value.parse::<i32>().unwrap_or(0);
            Expr::Number(val)
        }
        Some(t) if t.kind == TokenKind::IDENT => Expr::Ident(t.value.clone()),
        Some(t) if t.kind == TokenKind::STRING => {
            let clean_str = t.value.trim_matches('"').trim_matches('\'').to_string();
            Expr::String(clean_str)
        }
        other => panic!(
            "[ERROR.PARSER]: expected value (number, identifier, or string), got {:?}",
            other
        ),
    }
}

