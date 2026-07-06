use cao_lang::{compiler::compile, traits::into_f1, value::Value, vm::Vm};

fn parse(src: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_cao_lang::LANGUAGE.into())
        .unwrap();
    parser.parse(src, None).unwrap()
}

#[test]
fn fibonacci_recursive_end_to_end() {
    let src = include_str!("../test/fibonacci_program_recursive.caol");
    let tree = parse(src);
    let module = caolc::frontend::tree_to_program(&tree, src).expect("lower");
    let program = compile(module, None).expect("compile");

    struct State {
        printed: Option<i64>,
    }

    let f = |vm: &mut Vm<State>, v: Value| {
        if let Value::Integer(i) = v {
            vm.get_aux_mut().printed = Some(i);
        }
        Ok(Value::Nil)
    };

    let mut vm = Vm::new(State { printed: None }).unwrap();
    vm.max_instr = 1_000_000;
    vm.register_native_function("print", into_f1(f)).unwrap();
    vm.stack_push(Value::Integer(10)).unwrap();
    vm.run(&program).expect("run");

    assert_eq!(vm.unwrap_aux().printed, Some(89));
}
