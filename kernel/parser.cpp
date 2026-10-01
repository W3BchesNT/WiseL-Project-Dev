#include "header/parser.h"
#include <iostream>
#include <memory>

using namespace std;

// Check if character requires leading whitespace in reconstructed output
bool needs_space_before(const string& val) {
    if (val.empty()) return false;
    char c = val[0];
    return c != ',' && c != ']' && c != ')' && c != ':';
}

// Check if character requires trailing whitespace in reconstructed output
bool needs_space_after(const string& val) {
    if (val.empty()) return false;
    char c = val.back();
    return c != '[' && c != '(';
}

// Parse inline assembly block asm { ... }
ASTNode parse_asm_block(const vector<Token>& tokens, size_t& pos) {
    ASTNode node;
    node.type = NodeType::ASM_BLOCK;

    pos++; // Skip 'asm' keyword

    if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACE) {
        pos++;
        string current_line;
        int brace_depth = 0;

        while (pos < tokens.size()) {
            if (tokens[pos].type == TokenType::RBRACE && brace_depth == 0) {
                break;
            }

            if (tokens[pos].type == TokenType::LBRACE) {
                brace_depth++;
                if (!current_line.empty()) current_line += " ";
                current_line += "{";
                pos++;
                continue;
            }

            if (tokens[pos].type == TokenType::RBRACE && brace_depth > 0) {
                brace_depth--;
                current_line += "}";
                pos++;
                continue;
            }

            if (tokens[pos].type == TokenType::NEWLINE) {
                if (!current_line.empty()) {
                    node.asm_lines.push_back(current_line);
                    current_line.clear();
                }
                pos++;
                continue;
            }

            if (!tokens[pos].value.empty()) {
                if (!current_line.empty() && needs_space_before(tokens[pos].value) && needs_space_after(current_line)) {
                    current_line += " ";
                }
                current_line += tokens[pos].value;
            }
            pos++;
        }

        if (!current_line.empty()) {
            node.asm_lines.push_back(current_line);
        }

        if (pos < tokens.size() && tokens[pos].type == TokenType::RBRACE) {
            pos++;
        }
    }
    return node;
}

static int bin_prec(const string& op) {
    if (op == "*" || op == "/" || op == "%") return 2;
    if (op == "+" || op == "-") return 1;
    return -1;
}

static shared_ptr<ASTNode> parse_expr_from(const vector<Token>& tks, size_t& i, int min_prec);

static shared_ptr<ASTNode> parse_primary_from(const vector<Token>& tks, size_t& i) {
    if (i >= tks.size()) return nullptr;

    if (tks[i].type == TokenType::NUMBER) {
        auto n = make_shared<ASTNode>();
        n->type = NodeType::LITERAL;
        n->value = tks[i].value;
        i++;
        return n;
    }

    if (tks[i].type == TokenType::STRING && tks[i].value.size() == 3 && tks[i].value[0] == '\'') {
        auto n = make_shared<ASTNode>();
        n->type = NodeType::LITERAL;
        n->value = to_string((int)(unsigned char)tks[i].value[1]);
        i++;
        return n;
    }

    if (tks[i].type == TokenType::LPAREN) {
        i++;
        auto inner = parse_expr_from(tks, i, 0);
        if (i < tks.size() && tks[i].type == TokenType::RPAREN) i++;
        return inner;
    }

    if (tks[i].type == TokenType::IDENT) {
        string name = tks[i].value;
        i++;
        if (i < tks.size() && tks[i].type == TokenType::LBRACKET) {
            i++;
            auto idx = parse_expr_from(tks, i, 0);
            if (i < tks.size() && tks[i].type == TokenType::RBRACKET) i++;
            auto node = make_shared<ASTNode>();
            node->type = NodeType::INDEX_EXPR;
            node->left = make_shared<ASTNode>();
            node->left->type = NodeType::IDENT_REF;
            node->left->value = name;
            node->right = idx;
            return node;
        }
        auto n = make_shared<ASTNode>();
        n->type = NodeType::IDENT_REF;
        n->value = name;
        return n;
    }

    return nullptr;
}

static shared_ptr<ASTNode> parse_expr_from(const vector<Token>& tks, size_t& i, int min_prec) {
    shared_ptr<ASTNode> left;

    if (i < tks.size() && tks[i].type == TokenType::IDENT && tks[i].value == "-") {
        i++;
        auto operand = parse_expr_from(tks, i, 2);
        auto node = make_shared<ASTNode>();
        node->type = NodeType::BINARY_OP;
        node->op = "u-";
        node->right = operand;
        left = node;
    } else {
        left = parse_primary_from(tks, i);
    }

    if (!left) return nullptr;

    while (i < tks.size()) {
        int prec = bin_prec(tks[i].value);
        if (prec == -1 || prec < min_prec) break;
        string op = tks[i].value;
        i++;
        auto right = parse_expr_from(tks, i, prec + 1);
        if (!right) break;
        auto node = make_shared<ASTNode>();
        node->type = NodeType::BINARY_OP;
        node->op = op;
        node->left = left;
        node->right = right;
        left = node;
    }
    return left;
}

static void parse_rvalue_expr(const vector<Token>& tokens, size_t& pos, ASTNode& node, TokenType stop_token) {
    vector<Token> expr_tokens;
    while (pos < tokens.size() && tokens[pos].type != TokenType::NEWLINE && tokens[pos].type != stop_token) {
        expr_tokens.push_back(tokens[pos]);
        pos++;
    }
    size_t ei = 0;
    auto tree = parse_expr_from(expr_tokens, ei, 0);
    if (tree && ei == expr_tokens.size()) {
        node.expr = tree;
    } else {
        string val;
        for (const auto& t : expr_tokens) {
            if (!val.empty()) val += " ";
            val += t.value;
        }
        node.var_value = val;
    }
}

// Parse variable declarations (let / static)
ASTNode parse_let_stmt(const vector<Token>& tokens, size_t& pos, bool is_static, bool is_func_local) {
    ASTNode node;
    node.type = NodeType::LET_STMT;
    node.is_static = is_static;
    node.is_func_local = is_func_local;
    pos++;

    if (pos < tokens.size() && tokens[pos].type == TokenType::MUT) {
        node.is_mut = true;
        pos++;
    }
    if (pos < tokens.size() && tokens[pos].type == TokenType::IDENT) {
        node.var_name = tokens[pos].value;
        pos++;
    }
    if (pos < tokens.size() && tokens[pos].type == TokenType::COLON) {
        pos++;
    }
    if (pos < tokens.size() && tokens[pos].type == TokenType::IDENT) {
        node.var_type = tokens[pos].value;
        pos++;
    }
    // Parse pointer type definitions (*u8, *i32, etc.)
    if (node.var_type == "*" && pos < tokens.size() && tokens[pos].type == TokenType::IDENT) {
        node.var_type += tokens[pos].value;
        pos++;
    }
    // Parse array type definitions (str[], i32[], any[])
    if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACKET) {
        pos++;
        if (pos < tokens.size() && tokens[pos].type == TokenType::RBRACKET) {
            node.var_type += "[]";
            pos++;
        }
    }

    if (pos < tokens.size() && tokens[pos].type == TokenType::ASSIGN) {
        pos++;
        // Parse array literal ["a", "b", "c"]
        if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACKET) {
            pos++;
            vector<string> array_elements;
            while (pos < tokens.size() && tokens[pos].type != TokenType::RBRACKET) {
                if (tokens[pos].type == TokenType::COMMA) {
                    pos++;
                    continue;
                }
                if (tokens[pos].type == TokenType::STRING || tokens[pos].type == TokenType::NUMBER || tokens[pos].type == TokenType::IDENT) {
                    array_elements.push_back(tokens[pos].value);
                }
                pos++;
            }
            if (pos < tokens.size() && tokens[pos].type == TokenType::RBRACKET) {
                pos++;
            }
            // Store array elements in var_value as comma-separated
            string joined;
            for (size_t i = 0; i < array_elements.size(); i++) {
                if (i > 0) joined += ",";
                joined += array_elements[i];
            }
            node.var_value = "[" + joined + "]";
        }
        else {
            parse_rvalue_expr(tokens, pos, node, TokenType::NEWLINE);
        }
    }
    return node;
}

static ASTNode parse_block(const vector<Token>& tokens, size_t& pos);

static ASTNode parse_cond_and_body(const vector<Token>& tokens, size_t& pos, NodeType type, bool has_else = false) {
    ASTNode node;
    node.type = type;
    pos++;

    if (pos < tokens.size() && tokens[pos].type == TokenType::LPAREN) {
        pos++;
        string cond;
        while (pos < tokens.size() && tokens[pos].type != TokenType::RPAREN) {
            if (!cond.empty()) cond += " ";
            cond += tokens[pos].value;
            pos++;
        }
        if (pos < tokens.size() && tokens[pos].type == TokenType::RPAREN) pos++;
        node.condition = cond;
    }

    if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACE) {
        ASTNode body = parse_block(tokens, pos);
        node.body = body.body;
    }

    if (has_else && pos < tokens.size() && tokens[pos].type == TokenType::ELSE) {
        pos++;
        if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACE) {
            ASTNode else_body = parse_block(tokens, pos);
            node.else_body = else_body.body;
        }
    }

    return node;
}

// Parse function/block bodies and control flow statements
ASTNode parse_block(const vector<Token>& tokens, size_t& pos) {
    ASTNode block;
    block.type = NodeType::ASM_BLOCK;

    if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACE) {
        pos++;

        while (pos < tokens.size() && tokens[pos].type != TokenType::RBRACE) {
            if (tokens[pos].type == TokenType::ASM) {
                block.body.push_back(parse_asm_block(tokens, pos));
            }
            else if (tokens[pos].type == TokenType::LET) {
                block.body.push_back(parse_let_stmt(tokens, pos, false, true));
            }
            else if (tokens[pos].type == TokenType::STATIC) {
                block.body.push_back(parse_let_stmt(tokens, pos, true, true));
            }
            else if (tokens[pos].type == TokenType::RETURN) {
                ASTNode ret_node;
                ret_node.type = NodeType::RETURN_STMT;
                pos++;

                if (pos < tokens.size() && tokens[pos].type != TokenType::NEWLINE && tokens[pos].type != TokenType::RBRACE) {
                    parse_rvalue_expr(tokens, pos, ret_node, TokenType::RBRACE);
                }
                block.body.push_back(ret_node);
            }
            else if (tokens[pos].type == TokenType::WHILE) {
                block.body.push_back(parse_cond_and_body(tokens, pos, NodeType::WHILE_STMT));
            }
            else if (tokens[pos].type == TokenType::IF) {
                block.body.push_back(parse_cond_and_body(tokens, pos, NodeType::IF_STMT, true));
            }
            else if (tokens[pos].type == TokenType::BREAK) {
                ASTNode break_node;
                break_node.type = NodeType::BREAK_STMT;
                pos++;
                block.body.push_back(break_node);
            }

            else if (tokens[pos].type == TokenType::IDENT) {
                string name = tokens[pos].value;
                pos++;

                string target = name;
                if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACKET) {
                    target += "[";
                    pos++;
                    int depth = 1;
                    while (pos < tokens.size() && depth > 0) {
                        if (tokens[pos].type == TokenType::LBRACKET) depth++;
                        else if (tokens[pos].type == TokenType::RBRACKET) {
                            depth--;
                            if (depth == 0) { pos++; break; }
                        }
                        if (!tokens[pos].value.empty()) target += tokens[pos].value;
                        pos++;
                    }
                    target += "]";
                }

                if (pos < tokens.size() && tokens[pos].type == TokenType::ASSIGN) {
                    pos++;
                    ASTNode assign_node;
                    assign_node.type = NodeType::ASSIGN_STMT;
                    assign_node.var_name = target;
                    parse_rvalue_expr(tokens, pos, assign_node, TokenType::NEWLINE);
                    block.body.push_back(assign_node);
                }
                else if (pos < tokens.size() && tokens[pos].type == TokenType::PLUSPLUS) {
                    ASTNode inc_node;
                    inc_node.type = NodeType::INC_STMT;
                    inc_node.var_name = name;
                    pos++;
                    block.body.push_back(inc_node);
                }
                else {
                    ASTNode call_node;
                    call_node.type = NodeType::FUNC_CALL;
                    call_node.value = name;

                    if (pos < tokens.size() && tokens[pos].type == TokenType::LPAREN) {
                        pos++;
                        while (pos < tokens.size() && tokens[pos].type != TokenType::RPAREN) {
                            if (tokens[pos].type == TokenType::COMMA || tokens[pos].type == TokenType::NEWLINE) {
                                pos++;
                                continue;
                            }
                            if (tokens[pos].type == TokenType::LBRACKET) {
                                string arg_str = "[";
                                pos++;
                                int depth = 1;
                                while (pos < tokens.size() && depth > 0) {
                                    if (tokens[pos].type == TokenType::LBRACKET) depth++;
                                    else if (tokens[pos].type == TokenType::RBRACKET) {
                                        depth--;
                                        if (depth == 0) { pos++; break; }
                                    }
                                    if (tokens[pos].type == TokenType::STRING) {
                                        string val = tokens[pos].value;
                                        if (val.size() >= 2 && val[0] == '"' && val.back() == '"') {
                                            arg_str += val;
                                        } else {
                                            arg_str += "\"" + val + "\"";
                                        }
                                    } else if (tokens[pos].type == TokenType::COMMA) {
                                        arg_str += ",";
                                    } else if (!tokens[pos].value.empty()) {
                                        arg_str += tokens[pos].value;
                                    }
                                    pos++;
                                }
                                arg_str += "]";
                                call_node.args.push_back(arg_str);
                            }
                            else if (!tokens[pos].value.empty()) {
                                string arg_val = tokens[pos].value;
                                pos++;
                                if (arg_val == "&" && pos < tokens.size() && 
                                    (tokens[pos].type == TokenType::IDENT || tokens[pos].type == TokenType::NUMBER)) {
                                    arg_val += tokens[pos].value;
                                    pos++;
                                }
                                // Handle array indexing: name[index]
                                if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACKET) {
                                    arg_val += "[";
                                    pos++;
                                    int depth = 1;
                                    while (pos < tokens.size() && depth > 0) {
                                        if (tokens[pos].type == TokenType::LBRACKET) depth++;
                                        else if (tokens[pos].type == TokenType::RBRACKET) {
                                            depth--;
                                            if (depth == 0) { pos++; break; }
                                        }
                                        if (!tokens[pos].value.empty()) arg_val += tokens[pos].value;
                                        pos++;
                                    }
                                    arg_val += "]";
                                }
                                call_node.args.push_back(arg_val);
                            }
                            else {
                                pos++;
                            }
                        }
                        if (pos < tokens.size() && tokens[pos].type == TokenType::RPAREN) {
                            pos++;
                        }
                    }
                    block.body.push_back(call_node);
                }
            }
            else if (tokens[pos].type == TokenType::NEWLINE) {pos++;}
            else {
                pos++;
            }
        }
        if (pos < tokens.size() && tokens[pos].type == TokenType::RBRACE) {pos++;}
    }
    return block;
}

// Parse @data directives for assembly global data blocks
ASTNode parse_data_block(const vector<Token>& tokens, size_t& pos) {
    ASTNode node;
    node.type = NodeType::DATA_BLOCK;
    pos++;

    if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACE) {
        pos++;

        while (pos < tokens.size() && tokens[pos].type != TokenType::RBRACE) {
            if (tokens[pos].type == TokenType::ASM) {
                node.body.push_back(parse_asm_block(tokens, pos));
            }
            else {
                pos++;
            }
        }
        if (pos < tokens.size() && tokens[pos].type == TokenType::RBRACE) {
            pos++;
        }
    }
    return node;
}

// Parse @library directives for Windows DLL imports
ASTNode parse_dll_block(const vector<Token>& tokens, size_t& pos) {
    ASTNode node;
    node.type = NodeType::DLL_BLOCK;
    pos++;

    if (pos < tokens.size() && tokens[pos].type == TokenType::IDENT) {
        node.value = tokens[pos].value;
        pos++;
    }

    if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACE) {
        pos++;

        while (pos < tokens.size() && tokens[pos].type != TokenType::RBRACE) {
            
            if (tokens[pos].type == TokenType::NEWLINE) { pos++; continue;}

            if (!tokens[pos].value.empty()) {node.imports.push_back(tokens[pos].value);} pos++;
        }

        if (pos < tokens.size() && tokens[pos].type == TokenType::RBRACE) {
            pos++;
        }
    }
    return node;
}

// Parse function definitions
ASTNode parse_function(const vector<Token>& tokens, size_t& pos) {
    ASTNode node;
    node.type = NodeType::FUNC_DEF;
    pos++;

    if (pos < tokens.size() && tokens[pos].type == TokenType::IDENT) {
        node.value = tokens[pos].value; pos++;
    }

    if (pos < tokens.size() && tokens[pos].type == TokenType::LPAREN) {
        pos++;
        while (pos < tokens.size() && tokens[pos].type != TokenType::RPAREN) {
            if (tokens[pos].type == TokenType::COMMA) {
                pos++; continue;
            }
            if (tokens[pos].type == TokenType::IDENT) {
                string ptype = tokens[pos].value;
                pos++;
                if (ptype == "*" && pos < tokens.size() && tokens[pos].type == TokenType::IDENT) {
                    ptype += tokens[pos].value;
                    pos++;
                }
                if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACKET) {
                    pos++;
                    if (pos < tokens.size() && tokens[pos].type == TokenType::RBRACKET) {
                        ptype += "[]";
                        pos++;
                    }
                }

                if (pos < tokens.size() && tokens[pos].type == TokenType::IDENT) {
                    node.params.push_back(tokens[pos].value);
                    node.param_types.push_back(ptype);
                    pos++;
                }
                else {
                    node.params.push_back(ptype);
                    node.param_types.push_back(ptype);
                }
            }
            else {
                pos++;
            }
        }
        if (pos < tokens.size() && tokens[pos].type == TokenType::RPAREN) {
            pos++;
        }
    }

    if (pos < tokens.size() && tokens[pos].type == TokenType::ARROW) {
        pos++;
        if (pos < tokens.size() && tokens[pos].type == TokenType::IDENT) {
            node.return_type = tokens[pos].value;
            pos++;
        }
        if (node.return_type == "*" && pos < tokens.size() && tokens[pos].type == TokenType::IDENT) {
            node.return_type += tokens[pos].value;
            pos++;
        }
    }

    if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACE) {
        ASTNode body = parse_block(tokens, pos);
        for (const auto& stmt : body.body) {
            node.body.push_back(stmt);
        }
    }
    return node;
}

// Global AST Parser entry point
vector<ASTNode> parse(const vector<Token>& tokens) {
    vector<ASTNode> nodes;
    size_t pos = 0;

    while (pos < tokens.size() && tokens[pos].type != TokenType::END){
        if (tokens[pos].type == TokenType::FORMAT) {
            pos++;
            if (pos < tokens.size() && tokens[pos].type == TokenType::STRING) {
                ASTNode node;
                node.type = NodeType::FORMAT;
                node.value = tokens[pos].value;
                nodes.push_back(node);
                pos++;
            }
        }
        else if (tokens[pos].type == TokenType::FUNC) {
            nodes.push_back(parse_function(tokens, pos));
        }
        else if (tokens[pos].type == TokenType::DIRECTIVE && tokens[pos].value == "data") {
            nodes.push_back(parse_data_block(tokens, pos));
        }
        else if (tokens[pos].type == TokenType::DIRECTIVE && tokens[pos].value == "library") {
            nodes.push_back(parse_dll_block(tokens, pos));
        }
        else if (tokens[pos].type == TokenType::DIRECTIVE && tokens[pos].value == "include.inc") {
            ASTNode node;
            node.type = NodeType::INCLUDE_BLOCK;
            pos++;
            if (pos < tokens.size() && tokens[pos].type == TokenType::LBRACE) {
                pos++;
                while (pos < tokens.size() && tokens[pos].type != TokenType::RBRACE) {
                    if (tokens[pos].type == TokenType::STRING) {
                        node.imports.push_back(tokens[pos].value);
                    } pos++;
                }
                if (pos < tokens.size() && tokens[pos].type == TokenType::RBRACE) {
                    pos++;
                }
            }
            nodes.push_back(node);
        }
        else if (tokens[pos].type == TokenType::USELIB) {
            pos++;
            if (pos < tokens.size() && tokens[pos].type == TokenType::STRING) {
                ASTNode node;
                node.type = NodeType::USELIB;
                node.value = tokens[pos].value;
                nodes.push_back(node);
                pos++;
            }
        }
        else if (tokens[pos].type == TokenType::LET) {
            nodes.push_back(parse_let_stmt(tokens, pos, false, false));
        }
        else if (tokens[pos].type == TokenType::STATIC) {
            nodes.push_back(parse_let_stmt(tokens, pos, true, false));
        }
        else {
            pos++;
        }
    }
    return nodes;
}