#pragma once
#include "ast.h"
#include <string>
#include <vector>
#include <map>

// Generate ASM Block
void generate_asm_block(const ASTNode& node, std::string& code,
                        const std::map<std::string, std::string>& locals,
                        const std::map<std::string, std::string>& params,
                        const std::map<std::string, std::string>& var_types);

// Codegen Function
std::string generate(const std::vector<ASTNode>& nodes);