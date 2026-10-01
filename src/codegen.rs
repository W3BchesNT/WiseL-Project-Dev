use crate::parser::ASTNode;

pub fn generate(ast: &[ASTNode]) -> String {
    let mut asm = String::new();

    let mut format_str = None;
    for node in ast {
        if let ASTNode::Format(f) = node {
            format_str = Some(f);
        }
    }

    if let Some(fmt) = format_str {
        asm.push_str(&format!("format {}\n", fmt));
    }
    
    asm.push_str("entry start\n\n");
    asm.push_str("include 'win64w.inc'\n\n");

    asm.push_str("section '.text' code readable executable\n\n");
    asm.push_str("start:\n");

    for node in ast {
        if let ASTNode::Asm(code) = node {
            for line in code.lines() {
                asm.push_str(&format!("    {}\n", line.trim()));
            }
        }
    }

    asm.push_str("    sub rsp, 8\n");
    asm.push_str("    and rsp, -16\n");
    asm.push_str("    invoke ExitProcess, 0\n\n");

    asm.push_str("section '.idata' import data readable writeable\n");
    asm.push_str("    library kernel32, 'KERNEL32.DLL'\n");
    asm.push_str("    import kernel32, \\\n");
    asm.push_str("           ExitProcess, 'ExitProcess'\n");

    asm
}