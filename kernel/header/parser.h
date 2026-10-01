#pragma once
#include "lexer.h"
#include "ast.h"
#include <vector>

// Parser Function
std::vector<ASTNode> parse(const std::vector<Token>& tokens);