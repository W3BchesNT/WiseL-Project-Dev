#include "header/lexer.h"
#include <iostream>
#include <cctype>

using namespace std;

// Skip whitespace characters (space, tab, carriage return)
void skip_whitespace(const string& src, size_t& pos) {
    while (pos < src.size() && (src[pos] == ' ' || src[pos] == '\t' || src[pos] == '\r')) {pos++;}
}

// Read string literal including quotes
string read_string(const string& src, size_t& pos) {
    string result = "\"";
    pos++;
    while (pos < src.size() && src[pos] != '"') {result += src[pos];pos++;}
    result += "\"";
    if (pos < src.size()) pos++;
    return result;
}

// Read identifier (letters, digits, underscore)
string read_identifier(const string& src, size_t& pos) {
    string result;
    while (pos < src.size() && (isalnum(src[pos]) || src[pos] == '_')) {result += src[pos]; pos++;}
    return result;
}

// Read numeric literal (digits, optional hex suffix h/H)
string read_number(const string& src, size_t& pos) {
    string result;
    while (pos < src.size() && isdigit(src[pos])) {result += src[pos]; pos++;}
    if (pos < src.size() && (src[pos] == 'h' || src[pos] == 'H')) {
        result += src[pos];
        pos++;
    }
    return result;
}

// Map keyword string to token type
TokenType get_keyword_type(const string& word) {
    if (word == "func") return TokenType::FUNC;
    if (word == "if") return TokenType::IF;
    if (word == "else") return TokenType::ELSE;
    if (word == "while") return TokenType::WHILE;
    if (word == "break") return TokenType::BREAK;
    if (word == "let") return TokenType::LET;
    if (word == "mut") return TokenType::MUT;
    if (word == "asm") return TokenType::ASM;
    if (word == "static") return TokenType::STATIC;
    if (word == "return") return TokenType::RETURN;
    if (word == "Format") return TokenType::FORMAT;
    if (word == "UseLib") return TokenType::USELIB;
    return TokenType::IDENT;
}

struct OpToken { const char* text; TokenType type; };

static const OpToken OP_TABLE[] = {
    {"->", TokenType::ARROW},
    {"==", TokenType::EQ},
    {"++", TokenType::PLUSPLUS},
    {"<=", TokenType::IDENT},
    {">=", TokenType::IDENT},
    {"!=", TokenType::IDENT},
    {"=",  TokenType::ASSIGN},
    {"+",  TokenType::IDENT},
    {"-",  TokenType::IDENT},
    {"*",  TokenType::IDENT},
    {"<",  TokenType::IDENT},
    {">",  TokenType::IDENT},
    {"!",  TokenType::IDENT},
    {"/",  TokenType::DIV},
    {"%",  TokenType::MOD},
};

// Convert source code string into token vector
vector<Token> tokenize(const string& source) {
    vector<Token> tokens;
    size_t pos = 0;

    while (pos < source.size()) {
        skip_whitespace(source, pos);
        if (pos >= source.size()) break;

        char ch = source[pos];

        if (ch == '-' && pos + 1 < source.size() && isdigit(source[pos + 1])) {
            pos++;
            string num = "-" + read_number(source, pos);
            tokens.push_back({TokenType::NUMBER, num});
            continue;
        }

        if (ch == '/' && pos + 1 < source.size() && source[pos + 1] == '/') {
            while (pos < source.size() && source[pos] != '\n') pos++;
            continue;
        }

        bool op_matched = false;
        for (const auto& ot : OP_TABLE) {
            string op = ot.text;
            if (source.compare(pos, op.size(), op) == 0) {
                tokens.push_back({ot.type, op});
                pos += op.size();
                op_matched = true;
                break;
            }
        }
        if (op_matched) continue;

        switch (ch) {
            case '(':tokens.push_back({TokenType::LPAREN, "("});pos++;break;
            case ')':tokens.push_back({TokenType::RPAREN, ")"});pos++;break;

            case '{':tokens.push_back({TokenType::LBRACE, "{"});pos++;break;
            case '}':tokens.push_back({TokenType::RBRACE, "}"});pos++;break;

            case '[':tokens.push_back({TokenType::LBRACKET, "["});pos++;break;
            case ']':tokens.push_back({TokenType::RBRACKET, "]"});pos++;break;

            case ',':tokens.push_back({TokenType::COMMA, ","});pos++;break;
            case '\n':tokens.push_back({TokenType::NEWLINE, "\n"});pos++;break;
            case '"': tokens.push_back({TokenType::STRING, read_string(source, pos)}); break;
            case '\'': {
                string result = "'"; pos++;
                while (pos < source.size() && source[pos] != '\'') {
                    result += source[pos]; pos++;
                } 
                result += "'";
                if (pos < source.size()) pos++;
                tokens.push_back({TokenType::STRING, result}); break;
            }

            // preprocessors
            case '@': {
                pos++;
                if (pos < source.size() && isalpha(source[pos])) {
                    string word = read_identifier(source, pos);
                    while (pos < source.size() && source[pos] != ' ' && source[pos] != '\t' &&
                        source[pos] != '{' && source[pos] != '\n' && source[pos] != '\r') {
                        word += source[pos];
                        pos++;
                    }
                    tokens.push_back({TokenType::DIRECTIVE, word});
                }
                else { tokens.push_back({TokenType::IDENT, "@"}); }break;
            }
            case '.': {
                if (pos + 1 < source.size() && isdigit(source[pos+1])) {
                    pos++;
                    string num = "." + read_number(source, pos);
                    tokens.push_back({TokenType::NUMBER, num});
                }
                else if (pos + 1 < source.size() && (isalpha(source[pos+1]) || source[pos+1] == '_')) {
                    string label = ".";
                    pos++;
                    label += read_identifier(source, pos);
                    tokens.push_back({TokenType::IDENT, label});
                }
                else {
                    tokens.push_back({TokenType::DOT, "."});
                    pos++;
                }
                break;
            }
            
            case ':': {
                if (pos + 1 < source.size() && (isalpha(source[pos+1]) || source[pos+1] == '_')) {
                    string label = ":";
                    pos++;
                    label += read_identifier(source, pos);
                    tokens.push_back({TokenType::IDENT, label});
                }
                else {
                    tokens.push_back({TokenType::COLON, ":"});
                    pos++;
                }
                break;
            }

            default:
                if (isdigit(ch)) {
                    string num = read_number(source, pos);
                    tokens.push_back({TokenType::NUMBER, num});
                }
                else if (isalpha(ch) || ch == '_') {
                    string word = read_identifier(source, pos);
                    TokenType type = get_keyword_type(word);
                    tokens.push_back({type, word});
                }
                else {
                    tokens.push_back({TokenType::IDENT, string(1, ch)});
                    pos++;
                }
                break;
        }
    }

    tokens.push_back({TokenType::END, ""});
    return tokens;
}