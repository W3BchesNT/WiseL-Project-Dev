#include <iostream>
#include <sstream>
#include <algorithm>
#include "header/codegen.h"

using namespace std;

// Type registry
struct TypeInfo {
    const char* name;
    int size;
    bool is_signed;
    const char* fasm_dir;
    const char* load_op;
};

static const TypeInfo TYPE_TABLE[] = {
    {"i8",   1,  true,  "db", "movsx"},
    {"u8",   1,  false, "db", "movzx"},
    {"i16",  2,  true,  "dw", "movsx"},
    {"u16",  2,  false, "dw", "movzx"},
    {"i32",  4,  true,  "dd", "movsxd"},
    {"u32",  4,  false, "dd", "movzx"},
    {"i64",  8,  true,  "dq", "mov"},
    {"u64",  8,  false, "dq", "mov"},
    {"i128", 16, true,  "dq", "mov"},
    {"u128", 16, false, "dq", "mov"},
};

static const TypeInfo* lookup_type(const string& t) {
    for (const auto& ti : TYPE_TABLE) {
        if (t == ti.name) return &ti;
    }
    return nullptr;
}

static bool is_int_type(const string& t) { return lookup_type(t) != nullptr; }
static bool is_signed_type(const string& t) { const TypeInfo* ti = lookup_type(t); return ti && ti->is_signed; }
static bool is_unsigned_type(const string& t) { const TypeInfo* ti = lookup_type(t); return ti && !ti->is_signed; }
static bool is_pointer_type(const string& t) { return t.size() >= 2 && t[0] == '*'; }
static bool is_string_type(const string& t) { return t == "str" || t == "*u8" || t == "*i8" || t == "*char"; }
static bool is_float_type(const string& t) { return t == "f32" || t == "f64"; }
static bool is_char_type(const string& t) { return t == "char"; }

struct Target {
    const char* name;
    bool is64;
    int ptr;
    const char* ax;
    const char* cx;
    const char* dx;
    const char* si;
    const char* di;
    const char* sp;
    const char* bp;
    const char* ptr_kw;
    const char* ptr_dir;
    const char* fasm_format;
    bool args_in_regs;
    bool shadow_space;
    bool callee_cleans;
    bool align16;
    vector<string> pool;
    vector<string> arg_regs;
};

static Target WIN64 = {
    "win64", true, 8, 
    "rax", "rcx", "rdx", "rsi", "rdi", "rsp", "rbp", 
    "qword", "dq", "PE64 Console", 
    true, true, false, true, 
    {"rbx", "r12", "r13", "r14", "r15", "rdi", "rsi", "rbp"}, 
    {"rcx", "rdx", "r8", "r9"}, 
};

static Target TG = WIN64;

static int data_counter = 0; 
static string current_func_name; 
static map<string, string> func_return_types; 
static map<string, vector<string>> func_param_types;

static void emit(stringstream& ss, const string& instr) {
    ss << "    " << instr << "\n";
}

static void emit_label(stringstream& ss, const string& label) {
    ss << label << ":\n";
}

static void emit_comment(stringstream& ss, const string& comment) {
    ss << "    ; " << comment << "\n";
}

static bool is_num_lit(const string& v) { return !v.empty() && (isdigit(v[0]) || (v.size() > 1 && v[0] == '-' && isdigit(v[1]))); }

static string trim(string s) {
    size_t first = s.find_first_not_of(" \t\r\n");
    return (first == string::npos) ? "" : s.substr(first, s.find_last_not_of(" \t\r\n") - first + 1);
}

// Forward declarations
string resolve_var(const string& name, const map<string, string>& locals, 
                   const map<string, string>& params,
                   const map<string, string>& var_types);
                   
string type_to_fasm(const string& type);
string get_arg_type(const string& arg, const map<string, string>& locals, const map<string, string>& params, const map<string, string>& var_types, const map<string, string>& local_types);
string type_to_fasm(const string& type);

string escape_for_fasm(const string& raw) {
    string result;
    for (size_t i = 0; i < raw.size(); i++) {
        if (!result.empty()) result += ",";
        
        if (raw[i] == '\\' && i + 1 < raw.size()) {
            switch (raw[++i]) {
                case 'r': result += "13"; break;
                case 'n': result += "10"; break;
                case 't': result += "9";  break;
                case '"': result += "34"; break;
                case '\'': result += "39"; break;
                case '\\': result += "'\\'"; break;
                default: result += "'" + string(1, raw[i]) + "'"; break;
            }
        } else if (raw[i] == '\'') {
            result += "39";
        } else if (raw[i] == '"') {
            result += "34";
        } else {
            result += "'" + string(1, raw[i]) + "'";
        }
    }
    return result;
}

static void emit_string_literal(const string& quoted, const string& dest, stringstream& ss, stringstream& extra_ss) {
    string label = "str_arg_" + to_string(data_counter++);
    string raw = quoted.substr(1, quoted.size() - 2);
    emit(extra_ss, label + " db " + escape_for_fasm(raw) + ",0");
    emit(extra_ss, label + "_len dd " + to_string(raw.size()));
    emit(ss, "lea " + dest + ", [" + label + "]");
}

void generate_asm_block(const ASTNode& node, stringstream& ss,
                        const map<string, string>& locals,
                        const map<string, string>& params,
                        const map<string, string>& var_types) {
    for (const auto& line : node.asm_lines) {
        string result;
        size_t pos = 0;

        while (pos < line.size()) {
            size_t start = line.find('{', pos);
            if (start == string::npos) { result += line.substr(pos); break; }
            
            size_t close = line.find('}', start);
            if (close == string::npos) { result += line.substr(pos); break; }
            result += line.substr(pos, start - pos);
        
            string name = line.substr(start + 1, close - start - 1), clean;
            for (char c : name) if (c != ' ') clean += c;

            auto lit = locals.find(clean);
            auto pit = params.find(clean);
            result += (lit != locals.end()) ? lit->second : (pit != params.end() ? pit->second : clean);
            pos = close + 1;
        }
        emit(ss, trim(result));
    }
}

void generate_func_call(const ASTNode& stmt, stringstream& ss, stringstream& extra_ss, 
                        const map<string, string>& var_types, 
                        const map<string, string>& locals, 
                        const map<string, string>& local_types,
                        const map<string, string>& params,
                        const map<string, bool>& defined_functions) {
    if (stmt.args.empty()) {
        if (defined_functions.find(stmt.value) != defined_functions.end()) {
            emit(ss, "call func_" + stmt.value);
        } else {
            emit(ss, "call [" + stmt.value + "]");
        }
        return;
    }

    // NEW: Automatically construct any[] array on the stack at runtime
    auto fpt = func_param_types.find(stmt.value);
    if (fpt != func_param_types.end() && !fpt->second.empty() && fpt->second[0] == "any[]") {
        int num_args = stmt.args.size();
        int header_size = 8;
        int ptr_table_size = num_args * 8;
        int records_size = num_args * 16;
        int total_size = header_size + ptr_table_size + records_size;
        
        // Align to 16 bytes
        total_size = (total_size + 15) & ~15;
        
        emit(ss, "sub rsp, " + to_string(total_size));
        emit(ss, "mov r10, rsp");
        
        // Store length in header
        emit(ss, "mov qword [r10], " + to_string(num_args));
        
        int ptr_table_offset = header_size;
        int records_offset = header_size + ptr_table_size;
        
        for (int i = 0; i < num_args; i++) {
            const string& arg = stmt.args[i];
            string arg_type = get_arg_type(arg, locals, params, var_types, local_types);
            
            int record_offset = records_offset + (i * 16);
            int ptr_offset = ptr_table_offset + (i * 8);
            
            // Store pointer to the any record in the pointer table
            emit(ss, "lea r11, [r10 + " + to_string(record_offset) + "]");
            emit(ss, "mov [r10 + " + to_string(ptr_offset) + "], r11");
            
            if (arg_type == "str") {
                emit(ss, "mov qword [r10 + " + to_string(record_offset) + "], 1"); // type 1 = string
                
                if (arg.size() >= 2 && arg.front() == '"') {
                    string str_label = "str_any_" + to_string(data_counter++);
                    string raw = arg.substr(1, arg.size() - 2);
                    emit(extra_ss, str_label + " db " + escape_for_fasm(raw) + ",0");
                    emit(ss, "lea r11, [" + str_label + "]");
                    emit(ss, "mov [r10 + " + to_string(record_offset + 8) + "], r11");
                } else {
                    auto lit = locals.find(arg);
                    auto pit = params.find(arg);
                    if (lit != locals.end()) {
                        emit(ss, "mov [r10 + " + to_string(record_offset + 8) + "], " + lit->second);
                    } else if (pit != params.end()) {
                        emit(ss, "mov r11, " + pit->second);
                        emit(ss, "mov [r10 + " + to_string(record_offset + 8) + "], r11");
                    } else {
                        emit(ss, "lea r11, [" + arg + "]");
                        emit(ss, "mov [r10 + " + to_string(record_offset + 8) + "], r11");
                    }
                }
            } else {
                emit(ss, "mov qword [r10 + " + to_string(record_offset) + "], 0"); // type 0 = int
                
                if (is_num_lit(arg)) {
                    emit(ss, "mov qword [r10 + " + to_string(record_offset + 8) + "], " + arg);
                } else {
                    auto lit = locals.find(arg);
                    auto pit = params.find(arg);
                    if (lit != locals.end()) {
                        emit(ss, "mov [r10 + " + to_string(record_offset + 8) + "], " + lit->second);
                    } else if (pit != params.end()) {
                        emit(ss, "mov r11, " + pit->second);
                        emit(ss, "mov [r10 + " + to_string(record_offset + 8) + "], r11");
                    } else {
                        emit(ss, "mov r11, [" + arg + "]");
                        emit(ss, "mov [r10 + " + to_string(record_offset + 8) + "], r11");
                    }
                }
            }
        }
        
        // rcx gets the pointer to the start of the pointer table (which acts as the array data)
        emit(ss, "lea rcx, [r10 + " + to_string(header_size) + "]");
        if (defined_functions.find(stmt.value) != defined_functions.end()) {
            emit(ss, "call func_" + stmt.value);
        } else {
            emit(ss, "call [" + stmt.value + "]");
        }
        
        // Clean up stack
        emit(ss, "add rsp, " + to_string(total_size));
        return;
    }

    vector<string> arg_regs = TG.arg_regs;

    for (size_t i = 0; i < stmt.args.size() && i < arg_regs.size(); i++) {
        const string& arg = stmt.args[i];
        string reg = arg_regs[i];

        if (arg.size() >= 2 && arg.front() == '"') {
            emit_string_literal(arg, reg, ss, extra_ss);
        }
        else if (!arg.empty() && (isdigit(arg[0]) || (arg.size() > 1 && arg[0] == '-' && isdigit(arg[1])))) {
            emit(ss, "mov " + reg + ", " + arg);
        }
        else {
            if (arg.size() >= 2 && arg[0] == '&') {
                string varname = arg.substr(1);
                auto lit = locals.find(varname);
                auto pit = params.find(varname);
                auto tit = var_types.find(varname);
                
                if (lit != locals.end()) {
                    emit(ss, "sub rsp, 8");
                    emit(ss, "mov qword [rsp], " + lit->second);
                    emit(ss, "lea " + reg + ", [rsp]");
                } else if (pit != params.end()) {
                    emit(ss, "lea " + reg + ", [" + pit->second + "]");
                } else if (tit != var_types.end()) {
                    emit(ss, "lea " + reg + ", [" + varname + "]");
                } else {
                    emit(ss, "lea " + reg + ", [" + varname + "]");
                }
            }
            else {
                auto lit = locals.find(arg);
                auto pit = params.find(arg);
                auto tit = var_types.find(arg);

                if (lit != locals.end()) {
                    emit(ss, "mov " + reg + ", " + lit->second);
                } else if (pit != params.end()) {
                    emit(ss, "mov " + reg + ", " + pit->second);
                } else if (tit != var_types.end() && (is_string_type(tit->second) || tit->second == "i8" || tit->second == "u8")) {
                    emit(ss, "lea " + reg + ", [" + arg + "]");
                } else if (arg.size() >= 2 && arg.front() == '[' && arg.back() == ']') {
                    string content = arg.substr(1, arg.size() - 2);
                    vector<string> elements;
                    string current_elem;
                    int quote_count = 0;
                    for (size_t ci = 0; ci < content.size(); ci++) {
                        char c = content[ci];
                        if (c == '"') {
                            quote_count++;
                            current_elem += c;
                            if (quote_count == 2) {
                                elements.push_back(trim(current_elem));
                                current_elem.clear();
                                quote_count = 0;
                                if (ci + 1 < content.size() && content[ci+1] == ',') ci++;
                            }
                        } else if (quote_count == 0 && c == ',') {
                            continue;
                        } else {
                            current_elem += c;
                        }
                    }
                    if (!current_elem.empty() && quote_count == 0) elements.push_back(trim(current_elem));
                    
                    string arr_label = "arr_arg_" + to_string(data_counter++);
                    emit(extra_ss, arr_label + "_header:");
                    emit(extra_ss, "    dq " + to_string(elements.size()));
                    emit(extra_ss, arr_label + "_data:");

                    emit(extra_ss, "    dq " + arr_label + "_item_0");
                    for (size_t ei = 1; ei < elements.size(); ei++) {
                        emit(extra_ss, "    dq " + arr_label + "_item_" + to_string(ei));
                    }

                    for (size_t ei = 0; ei < elements.size(); ei++) {
                        string elem = elements[ei];
                        if (elem.size() >= 2 && elem[0] == '"' && elem.back() == '"') {
                            string raw = elem.substr(1, elem.size() - 2);
                            emit(extra_ss, arr_label + "_item_" + to_string(ei) + " db " + escape_for_fasm(raw) + ",0");
                        }
                    }
                    
                    emit(ss, "lea " + reg + ", [" + arr_label + "_data]");
                } else if (arg.find('[') != string::npos) {
                    size_t br = arg.find('[');
                    string base = arg.substr(0, br);
                    size_t cl = arg.find(']');
                    string index = arg.substr(br + 1, cl - br - 1);
                    
                    auto vit_arr = var_types.find(base);
                    auto lit_arr = locals.find(base);
                    auto ltit_arr = local_types.find(base);
                    auto pit_arr = params.find(base);
                    bool is_array = false;
                    string elem_type;
                    
                    if (vit_arr != var_types.end() && vit_arr->second.size() >= 2 && vit_arr->second.substr(vit_arr->second.size()-2) == "[]") {
                        is_array = true;
                        elem_type = vit_arr->second.substr(0, vit_arr->second.size() - 2);
                    }
                    if (!is_array && ltit_arr != local_types.end() && ltit_arr->second.size() >= 2 && ltit_arr->second.substr(ltit_arr->second.size()-2) == "[]") {
                        is_array = true;
                        elem_type = ltit_arr->second.substr(0, ltit_arr->second.size() - 2);
                    }
                    if (!is_array && pit_arr != params.end()) {
                        is_array = true;
                        elem_type = "str"; 
                    }
                    
                    if (is_array && (is_string_type(elem_type) || elem_type == "any")) {
                        string base_resolved = resolve_var(base, locals, params, var_types);
                        emit(ss, "mov rdx, " + base_resolved);
                        
                        if (is_num_lit(index)) {
                            emit(ss, "mov " + reg + ", [rdx + " + index + " * 8]");
                        } else {
                            string idx_resolved = resolve_var(index, locals, params, var_types);
                            emit(ss, "mov rax, " + idx_resolved);
                            emit(ss, "imul rax, 8");
                            emit(ss, "mov " + reg + ", [rdx + rax]");
                        }
                    } else {
                        string resolved = resolve_var(arg, locals, params, var_types);
                        emit(ss, "movzx " + reg + ", " + resolved);
                    }
                } else {
                    emit(ss, "mov " + reg + ", [" + arg + "]");
                }
            }
        }
    }

    if (stmt.args.size() > 4) {
        int stack_args = stmt.args.size() - 4;
        emit(ss, "sub rsp, " + to_string(stack_args * 8));
        for (size_t i = 4; i < stmt.args.size(); i++) {
            const string& arg = stmt.args[i];
            int offset = (i - 4) * 8;
            if (!arg.empty() && isdigit(arg[0])) {
                emit(ss, "mov qword [rsp + " + to_string(offset) + "], " + arg);
            } else if (arg == "NULL" || arg == "null" || arg == "0") {
                emit(ss, "mov qword [rsp + " + to_string(offset) + "], 0");
            } else {
                auto lit = locals.find(arg);
                auto pit = params.find(arg);
                auto tit = var_types.find(arg);
                if (lit != locals.end()) {
                    emit(ss, "mov qword [rsp + " + to_string(offset) + "], " + lit->second);
                } else if (pit != params.end()) {
                    emit(ss, "mov qword [rsp + " + to_string(offset) + "], " + pit->second);
                } else if (tit != var_types.end()) {
                    emit(ss, "mov qword [rsp + " + to_string(offset) + "], [" + arg + "]");
                } else {
                    emit(ss, "mov qword [rsp + " + to_string(offset) + "], [" + arg + "]");
                }
            }
        }
    }

    if (defined_functions.find(stmt.value) != defined_functions.end()) {
        emit(ss, "call func_" + stmt.value);
    } else {
        emit(ss, "call [" + stmt.value + "]");
    }

    if (stmt.args.size() > 4) {
        int stack_args = stmt.args.size() - 4;
        emit(ss, "add rsp, " + to_string(stack_args * 8));
    }
}

string resolve_var(const string& name, const map<string, string>& locals, 
                   const map<string, string>& params,
                   const map<string, string>& var_types) {
    size_t bracket = name.find('[');
    if (bracket != string::npos) {
        string base = name.substr(0, bracket);
        size_t close = name.find(']');
        string index = name.substr(bracket + 1, close - bracket - 1);
        
        string base_resolved = resolve_var(base, locals, params, var_types);
        
        auto idx_lit = locals.find(index);
        auto idx_pit = params.find(index);
        string idx_resolved;
        if (idx_lit != locals.end()) idx_resolved = idx_lit->second;
        else if (idx_pit != params.end()) idx_resolved = idx_pit->second;
        else idx_resolved = index;
        
        return "byte [" + base_resolved + " + " + idx_resolved + "]";
    }
    auto lit = locals.find(name);
    if (lit != locals.end()) return lit->second;
    auto pit = params.find(name);
    if (pit != params.end()) return pit->second;
    auto vit = var_types.find(name);
    if (vit != var_types.end()) {
        if (is_string_type(vit->second)) return name;
        string type = vit->second;
        if (type == "i8" || type == "u8" || type == "char") return "byte [" + name + "]";
        if (type == "i16" || type == "u16") return "word [" + name + "]";
        if (type == "i32" || type == "u32") return "dword [" + name + "]";
        if (type == "i64" || type == "u64") return "qword [" + name + "]";
        return "[" + name + "]";
    }
    return name;
}

static void emit_condition(const string& cond, stringstream& ss,
                           const map<string, string>& locals,
                           const map<string, string>& params,
                           const map<string, string>& var_types,
                           const string& jump_if_false_label,
                           stringstream& extra_ss,
                           const map<string, string>& local_types) {
    if (cond == "true") return;
    size_t pos;

    static const vector<pair<string, string>> ops = {
        {"<=", "jg"}, {">=", "jl"},
        {"!=", "je"}, {"==", "jne"},
        {"<",  "jge"}, {">",  "jle"},
    };

    for (const auto& op : ops) {
        pos = cond.find(op.first);
        if (pos != string::npos) {
            string left = trim(cond.substr(0, pos));
            string right = trim(cond.substr(pos + op.first.size()));

            size_t bracket_pos = left.find('[');
            if (bracket_pos != string::npos) {
                string base = trim(left.substr(0, bracket_pos));
                size_t close_pos = left.find(']');
                string index = trim(left.substr(bracket_pos + 1, close_pos - bracket_pos - 1));

                string base_reg = resolve_var(base, locals, params, var_types);
                string idx_reg = resolve_var(index, locals, params, var_types);

                if (base_reg.find('[') != string::npos) {
                    emit(ss, "mov rdx, " + base_reg);
                    base_reg = "rdx";
                }
                if (idx_reg.find('[') != string::npos) {
                    emit(ss, "mov rcx, " + idx_reg);
                    idx_reg = "rcx";
                }

                if (right.size() == 3 && right[0] == '\'' && right[2] == '\'') {
                    int char_val = (unsigned char)right[1];
                    emit(ss, "cmp byte [" + base_reg + " + " + idx_reg + "], " + to_string(char_val));
                } else {
                    emit(ss, "cmp byte [" + base_reg + " + " + idx_reg + "], " + right);
                }
                emit(ss, op.second + " " + jump_if_false_label);
                return;
            }
            else if ((op.first == "==" || op.first == "!=") && right.size() >= 2 && right[0] == '"') {
                string raw_str = right.substr(1, right.size() - 2);
                
                if (raw_str.length() == 1) {
                    int char_val = (unsigned char)raw_str[0];
                    string left_reg = resolve_var(left, locals, params, var_types);
                    
                    bool is_ptr = false;
                    auto vit = var_types.find(left);
                    if (vit != var_types.end() && (is_pointer_type(vit->second) || is_string_type(vit->second))) is_ptr = true;
                    auto lit = local_types.find(left);
                    if (lit != local_types.end() && (is_pointer_type(lit->second) || is_string_type(lit->second))) is_ptr = true;

                    if (is_ptr) {
                        emit(ss, "cmp byte [" + left_reg + "], " + to_string(char_val));
                    } else {
                        emit(ss, "cmp " + left_reg + ", " + to_string(char_val));
                    }
                    emit(ss, op.second + " " + jump_if_false_label);
                    return;
                }
                
                string label = "cmp_str_" + to_string(data_counter++);
                string cmp_label = "str_cmp_block_" + to_string(data_counter++);
                
                emit(extra_ss, label + " db " + escape_for_fasm(raw_str) + ",0");
                
                string left_reg = resolve_var(left, locals, params, var_types);
                emit_comment(ss, "String comparison");
                emit(ss, "lea rsi, [" + label + "]");
                emit(ss, "mov rdi, " + left_reg);
                emit_label(ss, "." + cmp_label + "_loop");
                emit(ss, "mov al, [rsi]");
                emit(ss, "mov cl, [rdi]");
                emit(ss, "cmp al, cl");
                emit(ss, "jne ." + cmp_label + "_diff");
                emit(ss, "test al, al");
                emit(ss, "jz ." + cmp_label + "_equal");
                emit(ss, "inc rsi");
                emit(ss, "inc rdi");
                emit(ss, "jmp ." + cmp_label + "_loop");
                
                emit_label(ss, "." + cmp_label + "_diff");
                if (op.first == "==") {
                    emit(ss, "jmp " + jump_if_false_label);
                } else {
                    emit_comment(ss, "strings different, != is true, continue");
                }
                emit(ss, "jmp ." + cmp_label + "_end");
                
                emit_label(ss, "." + cmp_label + "_equal");
                if (op.first == "!=") {
                    emit(ss, "jmp " + jump_if_false_label);
                } else {
                    emit_comment(ss, "strings equal, == is true, continue");
                }
                emit_label(ss, "." + cmp_label + "_end");
                return;
            }
            else if (right.size() == 3 && right[0] == '\'' && right[2] == '\'') {
                int char_val = (unsigned char)right[1];
                string left_reg = resolve_var(left, locals, params, var_types);
                
                bool is_string = false;
                auto vit = var_types.find(left);
                if (vit != var_types.end() && (is_pointer_type(vit->second) || is_string_type(vit->second))) {
                    is_string = true;
                }
                auto lit = local_types.find(left);
                if (lit != local_types.end() && (is_pointer_type(lit->second) || is_string_type(lit->second))) {
                    is_string = true;
                }
                
                if (is_string) {
                    emit(ss, "cmp byte [" + left_reg + "], " + to_string(char_val));
                } else {
                    emit(ss, "cmp " + left_reg + ", " + to_string(char_val));
                }
                emit(ss, op.second + " " + jump_if_false_label);
                return;
            }
            else {
                string left_reg = resolve_var(left, locals, params, var_types);
                string right_val = is_num_lit(right) ? right : resolve_var(right, locals, params, var_types);
                emit(ss, "cmp " + left_reg + ", " + right_val);
                emit(ss, op.second + " " + jump_if_false_label);
                return;
            }
        }
    }
}

string get_arg_type(const string& arg, const map<string, string>& locals, const map<string, string>& params, const map<string, string>& var_types, const map<string, string>& local_types) {
    if (arg.size() >= 2 && arg[0] == '"') return "str";
    if (is_num_lit(arg)) return "int";

    auto vit = var_types.find(arg);
    if (vit != var_types.end() && is_string_type(vit->second)) return "str";

    auto lit2 = local_types.find(arg);
    if (lit2 != local_types.end() && is_string_type(lit2->second)) return "str";

    return "int";
}

static int count_locals(const vector<ASTNode>& stmts) {
    int count = 0;
    for (const auto& stmt : stmts) {
        if (stmt.type == NodeType::LET_STMT) {
            if (!stmt.is_static) count++;
        }
        else if (stmt.type == NodeType::WHILE_STMT || stmt.type == NodeType::IF_STMT) {
            count += count_locals(stmt.body);
            count += count_locals(stmt.else_body);            
        }
    }
    return count;
}

static bool needs_frame(const vector<ASTNode>& stmts) {
    for (const auto& stmt : stmts) {
        if (stmt.type == NodeType::FUNC_CALL) return true;
        if (stmt.type == NodeType::LET_STMT && !stmt.var_value.empty() && stmt.var_value.find("(") != string::npos) return true;
        if (stmt.type == NodeType::ASM_BLOCK) {
            bool has_sub_rsp = false;
            for (const auto& line : stmt.asm_lines) if (line.find("sub rsp") != string::npos) has_sub_rsp = true;
            if (!has_sub_rsp) for (const auto& line : stmt.asm_lines) if (line.find("call") != string::npos || line.find("invoke") != string::npos) return true;
        }
        if (stmt.type == NodeType::WHILE_STMT || stmt.type == NodeType::IF_STMT) {
            if (needs_frame(stmt.body) || needs_frame(stmt.else_body)) return true;
        }
    }
    return false;
}

static void validate_type(const string& var_name, const string& var_type, 
                          const string& value, const map<string, string>& func_return_types_map) {
    if (value.size() >= 2 && value[0] == '"' && value.back() == '"') {
        if (is_int_type(var_type) || is_float_type(var_type)) {
            cerr << "[ERROR] Cannot assign string to numeric type '" << var_type 
                 << "' for variable '" << var_name << "'" << endl;
            exit(1);
        }
    }
    
    if (is_num_lit(value)) {
        if (is_string_type(var_type)) {
            cerr << "[ERROR] Cannot assign number to string type '" << var_type 
                 << "' for variable '" << var_name << "'" << endl;
            exit(1);
        }
    }
    
    if (value.find("(") != string::npos && value.find(")") != string::npos) {
        string func_name = trim(value.substr(0, value.find("(")));
        
        auto frt = func_return_types_map.find(func_name);
        if (frt != func_return_types_map.end()) {
            string ret_type = frt->second;
            bool ret_is_int = is_int_type(ret_type);
            bool var_is_str = is_string_type(var_type) || is_pointer_type(var_type);
            
            if (ret_is_int && var_is_str) {
                cerr << "[ERROR] Function '" << func_name << "' returns " << ret_type 
                     << " but variable '" << var_name << "' is " << var_type << endl;
                exit(1);
            }
        }
    }
}

// Added local_types to signature so INDEX_EXPR knows about parameter array types
static void emit_expr(const shared_ptr<ASTNode>& e, const string& dest,
                      stringstream& ss,
                      const map<string, string>& locals,
                      const map<string, string>& params,
                      const map<string, string>& var_types,
                      const map<string, string>& local_types) {
    if (!e) return;

    if (e->type == NodeType::LITERAL) {
        if (e->value == "0") emit(ss, "xor " + dest + ", " + dest);
        else emit(ss, "mov " + dest + ", " + e->value);
        return;
    }

    if (e->type == NodeType::IDENT_REF) {
        auto lit = locals.find(e->value);
        if (lit != locals.end()) { if (dest != lit->second) emit(ss, "mov " + dest + ", " + lit->second); return; }
        auto pit = params.find(e->value);
        if (pit != params.end()) { emit(ss, "mov " + dest + ", " + pit->second); return; }
        auto vit = var_types.find(e->value);
        if (vit != var_types.end() && is_string_type(vit->second)) {
            emit(ss, "lea " + dest + ", [" + e->value + "]");
            return;
        }
        emit(ss, "mov " + dest + ", " + resolve_var(e->value, locals, params, var_types));
        return;
    }

    if (e->type == NodeType::INDEX_EXPR) {
        emit_expr(e->right, "rax", ss, locals, params, var_types, local_types);
        
        string base_name = e->left->value;
        auto lit = locals.find(base_name);
        auto pit = params.find(base_name);
        auto vit = var_types.find(base_name);
        auto ltit = local_types.find(base_name);
        
        bool is_array = false;
        string elem_type = "";
        
        if (vit != var_types.end() && vit->second.size() >= 2 && vit->second.substr(vit->second.size()-2) == "[]") {
            is_array = true;
            elem_type = vit->second.substr(0, vit->second.size() - 2);
        }
        else if (ltit != local_types.end() && ltit->second.size() >= 2 && ltit->second.substr(ltit->second.size()-2) == "[]") {
            is_array = true;
            elem_type = ltit->second.substr(0, ltit->second.size() - 2);
        }
        
        if (is_array) {
            if (is_string_type(elem_type) || elem_type == "any") {
                if (lit != locals.end()) {
                    emit(ss, "mov rdx, " + lit->second);
                    emit(ss, "imul rax, 8");
                    emit(ss, "mov " + dest + ", [rdx + rax]");
                } else if (pit != params.end()) {
                    emit(ss, "mov rdx, " + pit->second);
                    emit(ss, "imul rax, 8");
                    emit(ss, "mov " + dest + ", [rdx + rax]");
                } else {
                    emit(ss, "imul rax, 8");
                    emit(ss, "mov " + dest + ", [" + base_name + " + rax]");
                }
            } else {
                int elem_size = 4;
                if (elem_type == "i8" || elem_type == "u8") elem_size = 1;
                else if (elem_type == "i16" || elem_type == "u16") elem_size = 2;
                else if (elem_type == "i64" || elem_type == "u64") elem_size = 8;
                
                if (lit != locals.end()) {
                    emit(ss, "imul rax, " + to_string(elem_size));
                    emit(ss, "mov " + dest + ", [" + lit->second + " + rax]");
                } else if (pit != params.end()) {
                    emit(ss, "imul rax, " + to_string(elem_size));
                    emit(ss, "mov " + dest + ", [" + pit->second + " + rax]");
                } else {
                    emit(ss, "imul rax, " + to_string(elem_size));
                    emit(ss, "mov " + dest + ", [" + base_name + " + rax]");
                }
            }
        } else {
            if (lit != locals.end()) {
                emit(ss, "movzx " + dest + ", byte [" + lit->second + " + rax]");
            } else if (pit != params.end()) {
                emit(ss, "mov rdx, " + pit->second);
                emit(ss, "movzx " + dest + ", byte [rdx + rax]");
            } else {
                emit(ss, "movzx " + dest + ", byte [" + base_name + " + rax]");
            }
        }
        return;
    }

    if (e->type == NodeType::BINARY_OP) {
        if (e->op == "u-") {
            emit_expr(e->right, dest, ss, locals, params, var_types, local_types);
            emit(ss, "neg " + dest);
            return;
        }

        if (e->left && e->right && 
            e->left->type == NodeType::LITERAL && 
            e->right->type == NodeType::LITERAL) {
            long long lv = atoll(e->left->value.c_str());
            long long rv = atoll(e->right->value.c_str());
            long long result = 0;
            if (e->op == "+") result = lv + rv;
            else if (e->op == "-") result = lv - rv;
            else if (e->op == "*") result = lv * rv;
            else if (e->op == "/" && rv != 0) result = lv / rv;
            else if (e->op == "%" && rv != 0) result = lv % rv;
            else goto no_fold;
            
            if (result == 0) emit(ss, "xor " + dest + ", " + dest);
            else emit(ss, "mov " + dest + ", " + to_string(result));
            return;
        }
        no_fold:

        if (e->op == "+" && e->right && e->right->type == NodeType::LITERAL && e->right->value == "0") {
            emit_expr(e->left, dest, ss, locals, params, var_types, local_types);
            return;
        }
        if (e->op == "-" && e->right && e->right->type == NodeType::LITERAL && e->right->value == "0") {
            emit_expr(e->left, dest, ss, locals, params, var_types, local_types);
            return;
        }
        if (e->op == "*" && e->right && e->right->type == NodeType::LITERAL && e->right->value == "1") {
            emit_expr(e->left, dest, ss, locals, params, var_types, local_types);
            return;
        }
        
        if (e->op == "-" && e->left && e->left->type == NodeType::LITERAL && e->left->value == "0") {
            emit_expr(e->right, dest, ss, locals, params, var_types, local_types);
            emit(ss, "neg " + dest);
            return;
        }

        string right_op;
        bool right_simple = false;
        if (e->right->type == NodeType::LITERAL) { right_op = e->right->value; right_simple = true; }
        else if (e->right->type == NodeType::IDENT_REF) {
            auto rl = locals.find(e->right->value);
            if (rl != locals.end()) { right_op = rl->second; right_simple = true; }
            else {
                auto rp = params.find(e->right->value);
                if (rp != params.end()) { right_op = rp->second; right_simple = true; }
            }
        }

        if (right_simple && (e->op == "+" || e->op == "-" || e->op == "*")) {
            emit_expr(e->left, dest, ss, locals, params, var_types, local_types);
            if (e->op == "+") emit(ss, "add " + dest + ", " + right_op);
            else if (e->op == "-") emit(ss, "sub " + dest + ", " + right_op);
            else emit(ss, "imul " + dest + ", " + right_op);
            return;
        }

        if (right_simple && (e->op == "/" || e->op == "%")) {
            emit(ss, "mov r10, " + right_op);
            emit_expr(e->left, "rax", ss, locals, params, var_types, local_types);
            emit(ss, "xor rdx, rdx");
            emit(ss, "div r10");
            if (e->op == "%") { if (dest != "rdx") emit(ss, "mov " + dest + ", rdx"); }
            else if (dest != "rax") emit(ss, "mov " + dest + ", rax");
            return;
        }

        if (dest != "rax") {
            if (e->op == "/" || e->op == "%") {
                emit_expr(e->right, "rax", ss, locals, params, var_types, local_types);
                emit(ss, "mov r10, rax");
                emit_expr(e->left, "rax", ss, locals, params, var_types, local_types);
                emit(ss, "xor rdx, rdx");
                emit(ss, "div r10");
                emit(ss, "mov " + dest + ", " + (e->op == "%" ? "rdx" : "rax"));
            } else {
                emit_expr(e->left, dest, ss, locals, params, var_types, local_types);
                emit_expr(e->right, "rax", ss, locals, params, var_types, local_types);
                if (e->op == "+") emit(ss, "add " + dest + ", rax");
                else if (e->op == "-") emit(ss, "sub " + dest + ", rax");
                else emit(ss, "imul " + dest + ", rax");
            }
            return;
        }

        emit_expr(e->left, "rax", ss, locals, params, var_types, local_types);
        emit(ss, "push rax");
        emit_expr(e->right, "rax", ss, locals, params, var_types, local_types);
        emit(ss, "mov rcx, rax");
        emit(ss, "pop rax");

        if (e->op == "+") emit(ss, "add rax, rcx");
        else if (e->op == "-") emit(ss, "sub rax, rcx");
        else if (e->op == "*") emit(ss, "imul rax, rcx");
        else if (e->op == "/") { emit(ss, "xor rdx, rdx"); emit(ss, "div rcx"); }
        else if (e->op == "%") { emit(ss, "xor rdx, rdx"); emit(ss, "div rcx"); emit(ss, "mov rax, rdx"); }

        if (dest != "rax") emit(ss, "mov " + dest + ", rax");
        return;
    }
}

void generate_block(const vector<ASTNode>& stmts, stringstream& ss, stringstream& extra_ss, 
                    const map<string, string>& var_types,
                    const map<string, string>& params,
                    map<string, string>& locals,
                    map<string, string>& local_types,
                    vector<string>& local_regs, int& local_index,
                    const string& loop_end_label,
                    const map<string, bool>& defined_functions) {
    
    for (const auto& stmt : stmts) {
        switch (stmt.type) {

            case NodeType::ASM_BLOCK: {
                generate_asm_block(stmt, ss, locals, params, var_types);
                break;
            }

            case NodeType::LET_STMT: {
                if (stmt.is_static) break;
                
                validate_type(stmt.var_name, stmt.var_type, stmt.var_value, func_return_types);
                
                string comment = "let " + stmt.var_name;
                if (!stmt.var_type.empty()) comment += ": " + stmt.var_type;
                if (!stmt.var_value.empty()) comment += " = " + stmt.var_value;
                emit_comment(ss, comment);

                if (local_index >= (int)local_regs.size()) {
                    cerr << "[ERROR] Too many local variables (max " << local_regs.size() << "). Variable: " << stmt.var_name << endl;
                    exit(1);
                }
                string reg = local_regs[local_index++];
                locals[stmt.var_name] = reg;
                if (!stmt.var_type.empty()) {
                    local_types[stmt.var_name] = stmt.var_type;
                }

                if ((is_string_type(stmt.var_type) || is_pointer_type(stmt.var_type)) && 
                    stmt.var_value.size() >= 2 && stmt.var_value[0] == '"') {
                    emit_string_literal(stmt.var_value, reg, ss, extra_ss);
                }
                else if (stmt.expr) {
                    if ((is_pointer_type(stmt.var_type) || is_string_type(stmt.var_type)) &&
                        stmt.expr->type == NodeType::IDENT_REF &&
                        var_types.find(stmt.expr->value) != var_types.end()) {
                        emit(ss, "lea " + reg + ", [" + stmt.expr->value + "]");
                    } else {
                        emit_expr(stmt.expr, reg, ss, locals, params, var_types, local_types);
                    }
                }
                else if (stmt.var_value.empty()) {
                    emit(ss, "xor " + reg + ", " + reg);
                }
                else if (stmt.var_value.find("(") != string::npos && stmt.var_value.find(")") != string::npos) {
                    string func_name = trim(stmt.var_value.substr(0, stmt.var_value.find("(")));

                    size_t open = stmt.var_value.find("(");
                    size_t close = stmt.var_value.rfind(")");
                    string args_str = stmt.var_value.substr(open + 1, close - open - 1);

                    vector<string> call_args;
                    string current;
                    int depth = 0;
                    for (size_t i = 0; i < args_str.size(); i++) {
                        char c = args_str[i];
                        if (c == '(' || c == '[') depth++;
                        else if (c == ')' || c == ']') depth--;
                        else if (c == ',' && depth == 0) {
                            string trimmed = trim(current);
                            if (!trimmed.empty()) call_args.push_back(trimmed);
                            current.clear();
                            continue;
                        }
                        current += c;
                    }
                    string trimmed = trim(current);
                    if (!trimmed.empty()) call_args.push_back(trimmed);

                    vector<string> arg_regs = TG.arg_regs;
                    for (size_t i = 0; i < call_args.size() && i < arg_regs.size(); i++) {
                        const string& arg = call_args[i];
                        string arg_reg = arg_regs[i];

                        if (arg.size() >= 2 && arg.front() == '"') {
                            emit_string_literal(arg, arg_reg, ss, extra_ss);
                        }
                        else if (is_num_lit(arg)) {
                            emit(ss, "mov " + arg_reg + ", " + arg);
                        }
                        else {
                            auto lit = locals.find(arg);
                            auto pit = params.find(arg);
                            auto tit = var_types.find(arg);
                            if (lit != locals.end()) {
                                emit(ss, "mov " + arg_reg + ", " + lit->second);
                            } else if (pit != params.end()) {
                                emit(ss, "mov " + arg_reg + ", " + pit->second);
                            } else if (tit != var_types.end() && is_string_type(tit->second)) {
                                emit(ss, "lea " + arg_reg + ", [" + arg + "]");
                            } else {
                                emit(ss, "lea " + arg_reg + ", [" + arg + "]");
                            }
                        }
                    }

                    if (defined_functions.find(func_name) != defined_functions.end()) {
                        emit(ss, "call func_" + func_name);
                    } else {
                        emit(ss, "call [" + func_name + "]");
                    }

                    auto frt = func_return_types.find(func_name);
                    if (frt != func_return_types.end() && !stmt.var_type.empty()) {
                        string return_type = frt->second;
                        bool func_returns_str = is_string_type(return_type) || is_pointer_type(return_type);
                        bool var_is_int = is_int_type(stmt.var_type);
                        
                        if (func_returns_str && var_is_int) {
                            emit(ss, "mov rcx, rax");
                            emit(ss, "call func_atoi");
                        }
                    }

                    emit(ss, "mov " + reg + ", rax");
                }
                else if (is_num_lit(stmt.var_value)) {
                    if (stmt.var_value == "0") emit(ss, "xor " + reg + ", " + reg);
                    else emit(ss, "mov " + reg + ", " + stmt.var_value);
                }
                else if (stmt.var_value.size() >= 2 && stmt.var_value.front() == '[' && stmt.var_value.back() == ']') {
                    string content = stmt.var_value.substr(1, stmt.var_value.size() - 2);
                    
                    vector<string> elements;
                    string current_elem;
                    int depth = 0;
                    for (size_t i = 0; i < content.size(); i++) {
                        char c = content[i];
                        if (c == '"' && (current_elem.empty() || current_elem.back() != '\\')) {
                            current_elem += c;
                            int quote_count = 0;
                            for (char ch : current_elem) if (ch == '"') quote_count++;
                            if (quote_count == 2 && (i + 1 >= content.size() || content[i+1] == ',')) {
                                elements.push_back(trim(current_elem));
                                current_elem.clear();
                                if (i + 1 < content.size() && content[i+1] == ',') i++;
                            }
                        } else if (c == ',' && depth == 0 && current_elem.find('"') == string::npos) {
                            if (!current_elem.empty()) {
                                elements.push_back(trim(current_elem));
                                current_elem.clear();
                            }
                        } else {
                            current_elem += c;
                        }
                    }
                    if (!current_elem.empty()) elements.push_back(trim(current_elem));
                    
                    string arr_label = "arr_arg_" + to_string(data_counter++);
                    string elem_type = stmt.var_type.substr(0, stmt.var_type.size() - 2); 
                    
                    emit(extra_ss, arr_label + "_header:");
                    emit(extra_ss, "    dq " + to_string(elements.size()));
                    emit(extra_ss, arr_label + "_data:");
                    
                    if (is_string_type(elem_type)) {
                        emit(extra_ss, "    dq " + arr_label + "_item_0");
                        for (size_t i = 1; i < elements.size(); i++) {
                            emit(extra_ss, "    dq " + arr_label + "_item_" + to_string(i));
                        }
                        for (size_t i = 0; i < elements.size(); i++) {
                            string elem = elements[i];
                            if (elem.size() >= 2 && elem[0] == '"' && elem.back() == '"') {
                                string raw = elem.substr(1, elem.size() - 2);
                                emit(extra_ss, arr_label + "_item_" + to_string(i) + " db " + escape_for_fasm(raw) + ",0");
                            }
                        }
                    } else {
                        string values;
                        for (size_t i = 0; i < elements.size(); i++) {
                            if (i > 0) values += ",";
                            values += elements[i];
                        }
                        emit(extra_ss, "    " + type_to_fasm(elem_type) + " " + values);
                    }
                    
                    emit(ss, "lea " + reg + ", [" + arr_label + "_data]");
                }
                else {
                    if (is_pointer_type(stmt.var_type) || is_string_type(stmt.var_type)) {
                        auto vit = var_types.find(stmt.var_value);
                        if (vit != var_types.end()) {
                            emit(ss, "lea " + reg + ", [" + stmt.var_value + "]");
                        } else {
                            string resolved = resolve_var(stmt.var_value, locals, params, var_types);
                            emit(ss, "mov " + reg + ", " + resolved);
                        }
                    }
                    else {
                        auto vit = var_types.find(stmt.var_value);
                        if (vit != var_types.end() && is_string_type(vit->second)) emit(ss, "mov " + reg + ", qword [" + stmt.var_value + "]");
                        else emit(ss, "mov " + reg + ", " + resolve_var(stmt.var_value, locals, params, var_types));
                    }
                }
                break;
            }

            case NodeType::ASSIGN_STMT: {
                string target = stmt.var_name;
                size_t br = target.find('[');

                if (br != string::npos) {
                    size_t cl = target.find(']');
                    string base = target.substr(0, br);
                    string index = target.substr(br + 1, cl - br - 1);

                    string idx_reg = resolve_var(index, locals, params, var_types);
                    if (idx_reg.find('[') != string::npos) {
                        emit(ss, "mov rcx, " + idx_reg);
                        idx_reg = "rcx";
                    }

                    string base_reg;
                    auto blit = locals.find(base);
                    if (blit != locals.end()) {
                        base_reg = blit->second;
                    } else {
                        auto bpit = params.find(base);
                        if (bpit != params.end()) {
                            emit(ss, "mov rdx, " + bpit->second);
                            base_reg = "rdx";
                        } else {
                            base_reg = base;
                        }
                    }

                    string mem = "byte [" + base_reg + " + " + idx_reg + "]";

                    if (stmt.expr) {
                        emit_expr(stmt.expr, "rax", ss, locals, params, var_types, local_types);
                        emit(ss, "mov " + mem + ", al");
                    } else {
                        string v = stmt.var_value;
                        if (v.size() == 3 && v[0] == '\'') {
                            emit(ss, "mov " + mem + ", " + to_string((int)(unsigned char)v[1]));
                        } else if (is_num_lit(v)) {
                            emit(ss, "mov " + mem + ", " + v);
                        } else {
                            emit(ss, "mov rax, " + resolve_var(v, locals, params, var_types));
                            emit(ss, "mov " + mem + ", al");
                        }
                    }
                }
                else {
                    auto lit = locals.find(target);
                    if (lit != locals.end()) {
                        if (stmt.expr) {
                            emit_expr(stmt.expr, lit->second, ss, locals, params, var_types, local_types);
                        } else {
                            string v = stmt.var_value;
                            if (!v.empty() && (isdigit(v[0]) || (v.size() > 1 && v[0] == '-' && isdigit(v[1])))) {
                                emit(ss, "mov " + lit->second + ", " + v);
                            } else {
                                emit(ss, "mov " + lit->second + ", " + resolve_var(v, locals, params, var_types));
                            }
                        }
                    } else {
                        if (stmt.expr) {
                            emit_expr(stmt.expr, "rax", ss, locals, params, var_types, local_types);
                            emit(ss, "mov [" + target + "], rax");
                        } else {
                            emit(ss, "mov rax, " + stmt.var_value);
                            emit(ss, "mov [" + target + "], rax");
                        }
                    }
                }
                break;
            }

            case NodeType::RETURN_STMT: {
                string val = stmt.var_value;
                if (!val.empty()) {
                    if (val.size() >= 2 && val[0] == '"') {
                        emit_string_literal(val, "rax", ss, extra_ss);
                    }
                    else if (is_num_lit(val)) {
                        emit(ss, "mov rax, " + val);
                    }
                    else if (val.find("(") != string::npos && val.find(")") != string::npos) {
                        string func_name = trim(val.substr(0, val.find("(")));
                        size_t open = val.find("(");
                        size_t close = val.rfind(")");
                        string args_str = val.substr(open + 1, close - open - 1);
                        
                        vector<string> call_args;
                        string current;
                        int depth = 0;
                        for (size_t i = 0; i < args_str.size(); i++) {
                            char c = args_str[i];
                            if (c == '(' || c == '[') depth++;
                            else if (c == ')' || c == ']') depth--;
                            else if (c == ',' && depth == 0) {
                                string trimmed = trim(current);
                                if (!trimmed.empty()) call_args.push_back(trimmed);
                                current.clear();
                                continue;
                            }
                            current += c;
                        }
                        string trimmed = trim(current);
                        if (!trimmed.empty()) call_args.push_back(trimmed);
                        
                        vector<string> arg_regs = TG.arg_regs;
                        for (size_t i = 0; i < call_args.size() && i < arg_regs.size(); i++) {
                            const string& arg = call_args[i];
                            string arg_reg = arg_regs[i];
                            
                            auto lit = locals.find(arg);
                            auto pit = params.find(arg);
                            auto tit = var_types.find(arg);
                            if (lit != locals.end()) {
                                emit(ss, "mov " + arg_reg + ", " + lit->second);
                            } else if (pit != params.end()) {
                                emit(ss, "mov " + arg_reg + ", " + pit->second);
                            } else if (tit != var_types.end() && is_string_type(tit->second)) {
                                emit(ss, "lea " + arg_reg + ", [" + arg + "]");
                            } else {
                                emit(ss, "lea " + arg_reg + ", [" + arg + "]");
                            }
                        }
                        
                        if (defined_functions.find(func_name) != defined_functions.end()) {
                            emit(ss, "call func_" + func_name);
                        } else {
                            emit(ss, "call [" + func_name + "]");
                        }
                    }
                    else {
                        auto type_it = var_types.find(val);
                        if (type_it != var_types.end() && is_string_type(type_it->second)) {
                            emit(ss, "lea rax, [" + val + "]");
                        }
                        else {
                            emit(ss, "mov rax, " + resolve_var(val, locals, params, var_types));
                        }
                    }
                }
                emit(ss, "jmp .func_end_" + current_func_name);
                break;
            }

            case NodeType::INC_STMT: {
                auto it = locals.find(stmt.var_name);
                if (it != locals.end()) {
                    emit_comment(ss, "increment " + stmt.var_name);
                    emit(ss, "inc " + it->second);
                }
                break;
            }

            case NodeType::BREAK_STMT: {
                if (!loop_end_label.empty()) {
                    emit_comment(ss, "break loop");
                    emit(ss, "jmp " + loop_end_label);
                }
                break;
            }

            case NodeType::WHILE_STMT: {
                static int while_counter = 0;
                string start_label = ".while_start_" + to_string(while_counter);
                string end_label = ".while_end_" + to_string(while_counter);
                while_counter++;

                emit_label(ss, start_label);
                emit_condition(stmt.condition, ss, locals, params, var_types, end_label, extra_ss, local_types);
                generate_block(stmt.body, ss, extra_ss, var_types, params, locals, local_types, local_regs, local_index, end_label, defined_functions);
                emit(ss, "jmp " + start_label);
                emit_label(ss, end_label);
                break;
            }

            case NodeType::IF_STMT: {
                static int if_counter = 0;
                string else_label = ".if_else_" + to_string(if_counter);
                string end_label = ".if_end_" + to_string(if_counter);
                if_counter++;

                emit_condition(stmt.condition, ss, locals, params, var_types, else_label, extra_ss, local_types);
                generate_block(stmt.body, ss, extra_ss, var_types, params, locals, local_types, local_regs, local_index, loop_end_label, defined_functions);

                if (!stmt.else_body.empty()) {
                    emit(ss, "jmp " + end_label);
                    emit_label(ss, else_label);
                    generate_block(stmt.else_body, ss, extra_ss, var_types, params, locals, local_types, local_regs, local_index, loop_end_label, defined_functions);
                }
                else {
                    emit_label(ss, else_label);
                }
                emit_label(ss, end_label);
                break;
            }

            case NodeType::FUNC_CALL: {
                string args_str;
                for (size_t i = 0; i < stmt.args.size(); i++) {
                    if (i > 0) args_str += ", ";
                    args_str += stmt.args[i];
                }
                emit_comment(ss, "call function " + stmt.value + "(" + args_str + ")");
                generate_func_call(stmt, ss, extra_ss, var_types, locals, local_types, params, defined_functions);
                break;
            }

            default:
                break;
        }
    }
}

void generate_function(const ASTNode& node, stringstream& ss, stringstream& extra_ss, const map<string, string>& var_types, const map<string, bool>& defined_functions) {
    emit_label(ss, "start");

    bool has_locals = count_locals(node.body) > 0;
    bool calls_funcs = needs_frame(node.body);

    vector<string> param_regs = TG.arg_regs;
    map<string, string> params;
    for (size_t i = 0; i < node.params.size() && i < param_regs.size(); i++) {
        params[node.params[i]] = param_regs[i];
    }

    int local_count = count_locals(node.body);
    vector<string> local_regs = TG.pool;
    int regs_needed = min(local_count, (int)local_regs.size());
    int local_index = 0;
    map<string, string> locals;
    map<string, string> local_types;

    if (has_locals || calls_funcs) {
        emit(ss, "sub rsp, 8");
        emit(ss, "and rsp, -16");
        for (int i = 0; i < regs_needed; i++) {
            emit(ss, "push " + local_regs[i]);
        }
        int alignment_padding = (regs_needed % 2 == 0) ? 8 : 0;
        int rsp_sub_size = 32 + alignment_padding;
        emit(ss, "sub rsp, " + to_string(rsp_sub_size));
    }

    current_func_name = "main";
    generate_block(node.body, ss, extra_ss, var_types, params, locals, local_types, local_regs, local_index, "", defined_functions);

    if (has_locals || calls_funcs) {
        int alignment_padding = (regs_needed % 2 == 0) ? 8 : 0;
        int rsp_sub_size = 32 + alignment_padding;
        emit(ss, "add rsp, " + to_string(rsp_sub_size));
        for (int i = regs_needed - 1; i >= 0; i--) {
            emit(ss, "pop " + local_regs[i]);
        }
    }
}

void generate_func_def(const ASTNode& node, stringstream& ss, stringstream& extra_ss, const map<string, string>& var_types, const map<string, bool>& defined_functions) {
    ss << "\n";
    emit_label(ss, "func_" + node.value);
    
    vector<string> param_regs = TG.arg_regs;
    map<string, string> params;
    for (size_t i = 0; i < node.params.size() && i < param_regs.size(); i++) {
        params[node.params[i]] = param_regs[i];
    }

    int local_count = count_locals(node.body);
    bool calls_funcs = needs_frame(node.body);
    vector<string> local_regs = TG.pool;
    int regs_needed = min(local_count, (int)local_regs.size());
    
    for (int i = 0; i < regs_needed; i++) {
        emit(ss, "push " + local_regs[i]);
    }

    int param_space = 0;
    if (node.params.size() > 0) {
        param_space = node.params.size() * 8;
        emit(ss, "sub rsp, " + to_string(param_space));
        for (size_t i = 0; i < node.params.size() && i < param_regs.size(); i++) {
            int offset = i * 8;
            emit(ss, "mov qword [rsp + " + to_string(offset) + "], " + param_regs[i]);
            params[node.params[i]] = "qword [rsp + " + to_string(offset) + "]";
        }
    }

    int local_index = 0;
    map<string, string> locals;
    map<string, string> local_types;

    // Store parameter types in local_types so emit_expr knows about them (e.g., any[])
    for (size_t i = 0; i < node.params.size() && i < node.param_types.size(); i++) {
        local_types[node.params[i]] = node.param_types[i];
    }

    current_func_name = node.value;
    generate_block(node.body, ss, extra_ss, var_types, params, locals, local_types, local_regs, local_index, "", defined_functions);

    emit_label(ss, ".func_end_" + node.value);

    if (node.params.size() > 0) {
        int param_space = node.params.size() * 8;
        emit(ss, "add rsp, " + to_string(param_space));
    }

    for (int i = regs_needed - 1; i >= 0; i--) {
        emit(ss, "pop " + local_regs[i]);
    }

    emit(ss, "ret");
}

string type_to_fasm(const string& type) {
    const TypeInfo* ti = lookup_type(type);
    if (ti) return ti->fasm_dir;
    if (type == "char") return "db";
    return "dq";
}

void generate_data_section(const vector<ASTNode>& nodes, stringstream& ss, const string& extra_data) {
    ss << "section '.data' data readable writeable\n";
    emit(ss, "wisel_v1 dd ?");
    for (const auto& node : nodes) {
        if (node.type == NodeType::DATA_BLOCK) {
            for (const auto& stmt : node.body) {
                if (stmt.type == NodeType::ASM_BLOCK) {
                    map<string, string> empty;
                    generate_asm_block(stmt, ss, empty, empty, empty);
                }
            }
        }
        else if (node.type == NodeType::LET_STMT && (node.is_static || !node.is_func_local)) {
            string name = node.var_name;
            string type = node.var_type;
            string value = node.var_value;

            if (is_string_type(type)) {
                string raw = value;
                if (raw.size() >= 2 && raw[0] == '"' && raw.back() == '"') {
                    raw = raw.substr(1, raw.size() - 2);
                }
                size_t display_len = 0;
                for (size_t i = 0; i < raw.size(); i++) {
                    if (raw[i] == '\\' && i + 1 < raw.size()) {
                        i++;
                    }
                    display_len++;
                }
                emit(ss, name + " db " + escape_for_fasm(raw) + ",0");
                emit(ss, name + "_len dd " + to_string(display_len));
            }
            else if (value.size() >= 2 && value.front() == '[' && value.back() == ']') {
                string content = value.substr(1, value.size() - 2);
                
                if (content.find(',') != string::npos || (content.size() > 0 && content.find("dup") == string::npos && isdigit(content[0]) == 0)) {
                    vector<string> elements;
                    string current;
                    for (size_t i = 0; i < content.size(); i++) {
                        if (content[i] == ',' ) {
                            if (!current.empty()) {
                                elements.push_back(current);
                                current.clear();
                            }
                        } else {
                            current += content[i];
                        }
                    }
                    if (!current.empty()) elements.push_back(current);
                    
                    emit(ss, name + "_len dd " + to_string(elements.size()));
                    
                    if (is_string_type(type.substr(0, type.size() - 2))) {
                        for (size_t i = 0; i < elements.size(); i++) {
                            string elem = elements[i];
                            if (elem.size() >= 2 && elem[0] == '"' && elem.back() == '"') {
                                string raw = elem.substr(1, elem.size() - 2);
                                emit(ss, name + "_item_" + to_string(i) + " db " + escape_for_fasm(raw) + ",0");
                            }
                        }
                        emit(ss, name + " dq " + to_string(elements.size()) + " dup(?)");
                    } else {
                        emit(ss, name + " " + type_to_fasm(type.substr(0, type.size() - 2)) + " " + to_string(elements.size()) + " dup(?)");
                    }
                }
                else {
                    string size_str = content;
                    string clean_size;
                    for (char c : size_str) {
                        if (c != ' ') clean_size += c;
                    }
                    emit(ss, name + " " + type_to_fasm(type) + " " + clean_size + " dup(?)");
                }
            }
            else {
                string fasm_value = value.empty() ? "?" : value;
                emit(ss, name + " " + type_to_fasm(type) + " " + fasm_value);
            }
        }
    }
    ss << extra_data;
    ss << "\n";
}

void generate_import_section(const vector<ASTNode>& nodes, stringstream& ss) {
    map<string, vector<string>> dll_imports;
    vector<string> dll_order;

    for (const auto& node : nodes) {
        if (node.type == NodeType::DLL_BLOCK) {
            string dll_name = node.value;
            if (dll_imports.find(dll_name) == dll_imports.end()) {
                dll_order.push_back(dll_name);
            }
            for (const auto& func : node.imports) {
                auto& imports = dll_imports[dll_name];
                bool found = false;
                for (const auto& existing : imports) {
                    if (existing == func) { found = true; break; }
                }
                if (!found) imports.push_back(func);
            }
        }
    }

    if (dll_order.empty()) return;

    string library_line = "    library ";
    string import_blocks = "";

    for (size_t d = 0; d < dll_order.size(); d++) {
        if (d > 0) library_line += ",\\\n            ";

        string dll_name = dll_order[d];
        string dll_upper = dll_name;
        for (auto& c : dll_upper) c = toupper(c);
        library_line += dll_name + ",'" + dll_upper + ".DLL'";
    }

    for (const auto& dll_name : dll_order) {
        import_blocks += "    import " + dll_name + ",\\\n";
        const auto& imports = dll_imports[dll_name];
        for (size_t i = 0; i < imports.size(); i++) {
            import_blocks += "        " + imports[i] + ",'" + imports[i] + "'";
            if (i < imports.size() - 1) import_blocks += ",\\";
            import_blocks += "\n";
        }
    }

    ss << "section '.idata' import data readable writeable\n";
    ss << library_line << "\n" << import_blocks;
}

string generate(const vector<ASTNode>& nodes) {
    string format_str;
    bool has_main = false;
    stringstream text_ss;
    stringstream extra_ss;
    
    data_counter = 0;
    func_return_types.clear();
    func_param_types.clear();

    map<string, string> var_types;
    map<string, bool> defined_functions;

    for (const auto& node : nodes) {
        if (node.type == NodeType::FUNC_DEF) {
            defined_functions[node.value] = true;
        }
    }

    for (const auto& node : nodes) {
        if (node.type == NodeType::FORMAT && format_str.empty()) {
            format_str = node.value;
        }
        else if (node.type == NodeType::FUNC_DEF) {
            if (!node.return_type.empty()) func_return_types[node.value] = node.return_type;
            if (!node.param_types.empty()) func_param_types[node.value] = node.param_types;
        }
        else if (node.type == NodeType::LET_STMT && (node.is_static || !node.is_func_local)) {
            var_types[node.var_name] = node.var_type;
        }
    }

    if (format_str.empty()) format_str = "PE64 Console";
    string clean_format = format_str;
    if (clean_format.size() >= 2 && clean_format.front() == '"' && clean_format.back() == '"') {
        clean_format = clean_format.substr(1, clean_format.size() - 2);
    }

    string low = clean_format;
    for (auto& c : low) c = tolower(c);
    if (low.find("pe64") != string::npos) TG = WIN64;
    else { cerr << "[ERROR] Unknown target: " << clean_format << " (use PE64 Console)" << endl; exit(1); }

    for (const auto& node : nodes) {
        if (node.type == NodeType::FUNC_DEF) {
            if (node.value == "main") {
                has_main = true;
                generate_function(node, text_ss, extra_ss, var_types, defined_functions);
            }
            else {
                generate_func_def(node, text_ss, extra_ss, var_types, defined_functions);
            }
        }
    }

    if (!has_main) {
        emit_label(text_ss, "start");
        emit(text_ss, "invoke ExitProcess, 0");
    }

    stringstream ss;
    ss << "format " << TG.fasm_format << "\n";
    ss << "entry start\n\n";
    
    for (const auto& node : nodes) {
        if (node.type == NodeType::INCLUDE_BLOCK) {
            for (const auto& inc : node.imports) {
                ss << "include '" << (inc.size() >= 2 && inc.front() == '"' && inc.back() == '"' ? inc.substr(1, inc.size() - 2) : inc) << "'\n";
            }
        }
    }
    ss << "\n"; 

    generate_data_section(nodes, ss, extra_ss.str());

    ss << "section '.text' code readable executable\n";
    ss << text_ss.str();
    ss << "\n";

    generate_import_section(nodes, ss);

    return ss.str();
}