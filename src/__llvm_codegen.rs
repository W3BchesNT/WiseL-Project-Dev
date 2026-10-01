use std::collections::HashMap;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::targets::TargetMachine;
use inkwell::values::{IntValue, PointerValue};
use inkwell::InlineAsmDialect;

use crate::parser::{ASTNode, Expr};

enum Variable<'ctx> {
    Immutable(IntValue<'ctx>),
    Mutable(PointerValue<'ctx>),
}

pub fn generate(ast: &[ASTNode]) -> Result<String, String> {
    let context = Context::create();
    let module = context.create_module("wisel");

    module.set_triple(&TargetMachine::get_default_triple());

    let builder = context.create_builder();
    let i32_type = context.i32_type();
    let main = module.add_function("main", i32_type.fn_type(&[], false), None);
    let entry = context.append_basic_block(main, "entry");

    builder.position_at_end(entry);

    let mut variables = HashMap::new();
    let mut returned = false;

    for node in ast {
        if returned {
            return Err("Unreachable statement after return".into());
        }
        match node {
            ASTNode::Let {
                mutable,
                name,
                ty,
                value,
            } => {
                if ty != "i32" {
                    return Err(format!("Unsupported LLVM type {ty:?}. Надо i32!!!!!!!!!!!!!!!!"));
                }

                if variables.contains_key(name) {
                    return Err(format!("Variable {name:?} is already declared"));
                }

                let value = expression(&context, &builder, &variables, value)?;
                let variable = if *mutable {
                    let pointer = builder
                        .build_alloca(i32_type, name)
                        .map_err(|e| e.to_string())?;
                    builder
                        .build_store(pointer, value)
                        .map_err(|e| e.to_string())?;
                    Variable::Mutable(pointer)
                } else { Variable::Immutable(value) };

                variables.insert(name.clone(), variable);
            }
            ASTNode::Asm(inline_code) => {
                // TODO: Convert FASM into ASM before feeding the inline code into LLVM IR?
                // Right now, эээ хз в общем завтра
                continue;

                // Create a function signature that will be a template for inlining assembly
                let asm_type = context.void_type().fn_type(&[], false);
                let asm = context.create_inline_asm(
                    asm_type,
                    inline_code.to_owned(),
                    // 'constraints'
                    String::new(),
                    true,
                    false,
                    Some(InlineAsmDialect::Intel),
                    false,
                );

                builder
                    .build_indirect_call(asm_type, asm, &[], "")
                    .map_err(|error| error.to_string())?;
            },
            ASTNode::Return(value) => {
                let value = expression(&context, &builder, &variables, value)?;
                builder
                    .build_return(Some(&value))
                    .map_err(|e| e.to_string())?;
                returned = true;
            }
            // TODO: later
            _ => {}
        }
    }

    if !returned {
        builder
            .build_return(Some(&i32_type.const_zero()))
            .map_err(|e| e.to_string())?;
    }

    module
        .verify()
        .map_err(|e| format!("LLVM verification failed: {e}"))?;

    Ok(module.print_to_string().to_string())
}

fn expression<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    variables: &HashMap<String, Variable<'ctx>>,
    expr: &Expr,
) -> Result<IntValue<'ctx>, String> {
    match expr {
        Expr::Number(value) => Ok(context.i32_type().const_int(*value as u64, true)),
        Expr::Ident(name) => match variables.get(name) {
            Some(Variable::Immutable(value)) => Ok(*value),
            Some(Variable::Mutable(pointer)) => builder
                .build_load(context.i32_type(), *pointer, &format!("{name}.value"))
                .map(|value| value.into_int_value())
                .map_err(|e| e.to_string()),
            None => Err(format!("Undefined variable {name:?}")),
        },
    }
}
