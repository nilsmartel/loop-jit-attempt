mod structure;
use std::collections::HashMap;

use cranelift::prelude::{types::I64, AbiParam, EntityRef, ExternalName, InstBuilder, IntCC};
use cranelift_codegen::{binemit::{NullStackMapSink, NullTrapSink}, ir::Value};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{default_libcall_names, Module};
use structure::Program;

use cranelift_jit::{JITBuilder, JITModule};
fn main() {
    let program: Program = todo!();

    let program = compile(program);
}

fn compile(program: Program) -> fn(f64) -> f64 {
    let mut module = {
        let builder = JITBuilder::new(default_libcall_names());
        JITModule::new(builder)
    };
    let sign = {
        let mut sign = module.make_signature();
        sign.params.push(AbiParam::new(I64));
        sign.returns.push(AbiParam::new(I64));
        sign
    };
    let func_id = module.declare_anonymous_function(&sign).unwrap();
    let mut context = module.make_context();
    context.func.signature = sign;
    context.func.name = ExternalName::User {
        namespace: 0,
        index: func_id.as_u32(),
    };
    {

        let mut fctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut context.func, &mut fctx);

        let vars: HashMap<String, Variable> = program
            .variables
            .into_iter()
            .zip(0usize..)
            .map(|(name, i)| {
                let var = Variable::new(i);

                builder.declare_var(var, I64);
                (name, var)
            })
            .collect();

        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);

        // Set variable input to argument of function
        if let Some(var) = vars.get("input") {
            builder.def_var(*var, builder.block_params(entry)[0]);
        }

        jit(&mut builder, &vars, &program.instructions);

        // return value stored in output OR 0 if output is not set
        let retval = if let Some(var) = vars.get("output") {
            builder.use_var(*var)
        } else {
            builder.ins().iconst(I64, 0)
        };
        builder.ins().return_(&[retval]);

        // finish up
        builder.seal_all_blocks();
        builder.finalize();
    }
    let mut trap_sink = NullTrapSink {};
    let mut stack_map_sink = NullStackMapSink {};
    module.define_function(func_id, &mut context, &mut trap_sink, &mut stack_map_sink).unwrap();
    module.clear_context(&mut context);
    module.finalize_definitions();
    let ptr = module.get_finalized_function(func_id);

    unsafe {
        std::mem::transmute::<*const u8, fn(f64) -> f64>(ptr)
    }
}

fn jit(
    mut builder: &mut FunctionBuilder,
    vars: &HashMap<String, Variable>,
    instructions: &[structure::Instruction],
) {
    if instructions.len() == 0 {
        return;
    }

    let instr = &instructions[0];
    let instructions = &instructions[1..];

    use structure::Instruction::*;
    match instr {
        &Assign { ref to, ref expr } => {
            let var = vars.get(to).unwrap().clone();
            let value = eval(expr, &mut builder, &vars);
            builder.def_var(var, value);
        }
        &If {
            ref condition,
            ref body,
        } => {
            let condition = eval(condition, &mut builder, &vars);
            let ifblock = builder.create_block();
            let continueblock = builder.create_block();

            // TODO 0 or 1 here? -> verify what the result of an comparision is
            let success = builder.ins().iconst(I64, 1);

            builder
                .ins()
                .br_icmp(IntCC::Equal, condition, success, ifblock, &[]);
            builder.ins().jump(continueblock, &[]);
            // TODO? seal current block

            builder.switch_to_block(ifblock);
            jit(&mut builder, vars, body);
            builder.seal_block(ifblock);

            // Progress to the next block
            builder.switch_to_block(continueblock);
        }
        _ => unimplemented!(),
    }

    jit(builder, vars, instructions)
}

fn eval(
    expr: &structure::Expr,
    mut builder: &mut FunctionBuilder,
    vars: &HashMap<String, Variable>,
) -> Value {
    let left = val(&expr.left, &mut builder, vars);
    let right = val(&expr.right, &mut builder, vars);

    use structure::Operation::*;
    match expr.op {
        Plus => builder.ins().iadd(left, right),
        Minus => builder.ins().isub(left, right),
        Times => builder.ins().imul(left, right),
        // TODO can this right?
        Divided => builder.ins().udiv(left, right),
        Equal => builder.ins().icmp(IntCC::Equal, left, right),
        NotEqual => builder.ins().icmp(IntCC::NotEqual, left, right),
        Modulo => unimplemented!(),
    }
}

fn val(
    value: &structure::Value,
    builder: &mut FunctionBuilder,
    vars: &HashMap<String, Variable>,
) -> Value {
    match value {
        &structure::Value::Constant(n) => builder.ins().iconst(I64, n),
        &structure::Value::Variable(ref name) => {
            let var = vars[name];
            builder.use_var(var)
        }
    }
}
