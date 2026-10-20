use crate::lexer::Token;
use crate::parser::Expr;

pub fn expect(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
    matches_fn: fn(&Token) -> bool,
    description: &str,
) -> Token {
    match iter.next() {
        Some(t) if matches_fn(t) => t.clone(),
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
        Some(Token::Identifier(s)) => s.clone(),
        other => panic!(
            "[ERROR.PARSER]: expected {}, got {:?}",
            description, other
        ),
    }
}

pub fn skip_if(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
    matches_fn: fn(&Token) -> bool,
) -> bool {
    if let Some(t) = iter.peek() {
        if matches_fn(t) {
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
        Some(Token::WholeNumber(s)) | Some(Token::FloatNumber(s)) => {
            let val = s
                .trim_end_matches(|c: char| c == 'h' || c == 'H')
                .parse::<i32>()
                .unwrap_or(0);
            Expr::Number(val)
        }
        Some(Token::Identifier(s)) => Expr::Ident(s.clone()),
        Some(Token::StringLiteral(s)) => Expr::String(s.clone()),
        other => panic!(
            "[ERROR.PARSER]: expected value (number, identifier, or string), got {:?}",
            other
        ),
    }
}