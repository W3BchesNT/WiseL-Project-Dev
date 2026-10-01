#pragma once
#include <string>
#include <vector>


// Format "_format"
enum class TokenType {
    FORMAT,
    STRING,
    
    FUNC,
    ASM,
    IDENT,

    LET,
    MUT,
    COLON,
    ASSIGN,
    STATIC,

    LPAREN,
    RPAREN,
    LBRACE,
    RBRACE,
    LBRACKET,
    RBRACKET,

    COMMA,
    NUMBER,

    DIRECTIVE,
    NEWLINE,

    WHILE,
    IF,
    ELSE,
    BREAK,

    EQ,
    PLUSPLUS,
    DOT,

    USELIB,
    
    DIV,
    MOD,
    
    ARROW,
    
    RETURN,
    END
};

// Token Structure
struct Token {
    TokenType type;
    std::string value;
};

// Lexer Function
std::vector<Token> tokenize(const std::string& source);