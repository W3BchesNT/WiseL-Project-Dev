use crate::lexer::Token;
use crate::parser::{BinaryOp, Expr};
use crate::parser_helpers::{expect, parse_value};

pub fn parse_expr(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
) -> Expr {
    parse_expr_bp(iter, 0)
}

fn parse_expr_bp(
    iter: &mut std::iter::Peekable<std::slice::Iter<'_, Token>>,
    min_bp: u8,
) -> Expr {
    // Parse the initial expression
    let mut left = match iter.peek() {
        Some(Token::LParen) => {
            iter.next(); // Consume '('

            let inner = parse_expr_bp(iter, 0);

            expect(iter, Token::RParen, "')'");
            inner
        }

        _ => parse_value(iter),
    };

    loop {
        let (op, left_bp, right_bp) = match iter.peek() {
            Some(Token::Plus) => (BinaryOp::Add, 1, 2),
            Some(Token::Minus) => (BinaryOp::Sub, 1, 2),

            _ => break,
        };

        match left_bp {
            bp if bp < min_bp => break,
            _ => {}
        }

        iter.next(); // Consume '+', -'

        let right = parse_expr_bp(iter, right_bp);

        left = Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        };
    }

    left
}
