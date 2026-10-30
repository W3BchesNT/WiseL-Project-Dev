use crate::lexer::Token;
use crate::parser::{BinaryOp, Expr};
use crate::parser_helpers::{expect, parse_value};

pub fn parse_expr(iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>) -> Expr {
    parse_expr_bp(iter, 0)
}

fn get_infix(token: &Token) -> Option<(BinaryOp, u8, u8)> {
    match token {
        Token::Plus            => Some((BinaryOp::Add, 1, 2)),
        Token::Minus           => Some((BinaryOp::Sub, 1, 2)),
        Token::Star            => Some((BinaryOp::Mul, 3, 4)),
        Token::Div             => Some((BinaryOp::Div, 3, 4)),
        Token::Mod             => Some((BinaryOp::Mod, 3, 4)),
        Token::Eq              => Some((BinaryOp::Eq, 5, 6)),
        Token::InEq            => Some((BinaryOp::Ne, 5, 6)),
        Token::LessThan        => Some((BinaryOp::Lt, 5, 6)),
        Token::GreaterThan     => Some((BinaryOp::Gt, 5, 6)),
        Token::LessThanOrEq    => Some((BinaryOp::Le, 5, 6)),
        Token::GreaterThanOrEq => Some((BinaryOp::Ge, 5, 6)),
        _ => None,
    }
}

fn parse_expr_bp(iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>, min_bp: u8) -> Expr {
    let mut left = match iter.peek() {
        Some(Token::LParen) => {
            iter.next();
            let inner = parse_expr_bp(iter, 0);
            expect(iter, Token::RParen, "')'");
            inner
        }
        _ => parse_value(iter),
    };

    loop {
        let (op, left_bp, right_bp) = match iter.peek() {
            Some(tok) => match get_infix(tok) {
                Some(infix) => infix,
                None => break,
            },
            None => break,
        };

        if left_bp < min_bp { break; }

        iter.next();
        let right = parse_expr_bp(iter, right_bp);
        left = Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        };
    }
    left
}