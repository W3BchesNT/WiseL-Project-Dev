#include <iostream>
#include <fstream>
#include <sstream>
#include "header/lexer.h"
#include "header/parser.h"
#include "header/codegen.h"

using namespace std;

// step 1: read the file 
string read_file(const string& path) {
    ifstream f(path);
    if(!f.is_open()) {
        cerr << "[ERROR] Cannot open: " << path << endl;
        exit(1);
    }
    stringstream ss;
    ss << f.rdbuf();
    return ss.str();
}

int main() {
    // 1. Read the Source
    string source = read_file("main.wise");
    cout << "[1/4] Read main.wise (" << source.size() << " bytes)" << endl;

    // 2. Lexer: text -> tokens
    vector<Token> tokens = tokenize(source);
    cout << "[2/4] Tokenized: " << tokens.size() << " tokens" << endl;

    // 3. Parser: tokens -> AST
    vector<ASTNode> ast = parse(tokens);
    cout << "[3/4] Parser: " << ast.size() << " AST Nodes" << endl;

    // 3.1 UseLib:
    vector<ASTNode> final_ast;
    for (const auto& node : ast) {
        if (node.type == NodeType::USELIB) {
            string lib_path = node.value;

            if (lib_path.size() >= 2 && lib_path.front() == '"' && lib_path.back() == '"') {
                lib_path = lib_path.substr(1, lib_path.size() - 2);
            }

            string lib_source = read_file(lib_path);
            vector<Token> lib_tokens = tokenize(lib_source);
            vector<ASTNode> lib_ast = parse(lib_tokens);
            for (const auto& lib_node : lib_ast) {
                final_ast.push_back(lib_node);
            }
        }
        else {
            final_ast.push_back(node);
        }
    }

    // 3.2 Displaying the function list
    for (const auto& node : final_ast) {
        if (node.type == NodeType::FUNC_DEF) {
            cout << "  func " << node.value << "(";
            for (size_t i = 0; i < node.params.size(); i++) {
                cout << node.params[i];
                if (i < node.params.size() - 1) cout << ", ";
            }
            cout << ")" << endl;
        }
    }

    // 4. Codegen: AST -> FASM
    string asm_code = generate(final_ast);
    ofstream out("out.asm");
    out << asm_code;
    out.close();

    cout << "[4/4] Generated out.asm (" << asm_code.size() << " bytes)" << endl;
    cout << "\nRun: fasm.exe out.asm main.exe && main.exe" << endl;

    return 0;
}