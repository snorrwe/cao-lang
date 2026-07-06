use cao_lang::{compiler::compile, value::Value, vm::Vm};

fn parse(src: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_cao_lang::LANGUAGE.into())
        .unwrap();
    parser.parse(src, None).unwrap()
}

#[test]
fn table_literal_desugars_and_compiles() {
    let src = r#"
        fn main() {
            global t = #{a: 1, "b-c": 2};
        }
    "#;
    let tree = parse(src);
    let module = caolc::frontend::tree_to_program(&tree, src).expect("lower");
    let program = compile(module, None).expect("compile");

    let mut vm = Vm::new(()).unwrap();
    vm.run(&program).expect("run");

    let t = vm
        .read_var_by_name("t", &program.variables)
        .expect("t not found");
    assert!(matches!(t, Value::Object(_)));
}

#[test]
fn known_callee_becomes_static_call() {
    // `helper` is declared alongside `main`, so it must be resolved as a static `Call` at
    // compile time -- if it were (mis-)treated as a native call, this would compile fine but
    // fail at runtime looking for a registered native function named "helper".
    let src = r#"
        fn main() {
            global result = helper();
        }
        fn helper() {
            return 42;
        }
    "#;
    let tree = parse(src);
    let module = caolc::frontend::tree_to_program(&tree, src).expect("lower");
    let program = compile(module, None).expect("compile");

    let mut vm = Vm::new(()).unwrap();
    vm.run(&program).expect("run");

    let result = vm
        .read_var_by_name("result", &program.variables)
        .expect("result not found");
    assert_eq!(result, Value::Integer(42));
}

#[test]
fn unknown_callee_becomes_native_call_and_fails_without_registration() {
    // `mystery` is never declared as a cao-lang function, so it must be lowered as `CallNative`
    // -- unregistered, this fails at *run* time (not compile time), confirming the branch taken.
    let src = r#"
        fn main() {
            mystery();
        }
    "#;
    let tree = parse(src);
    let module = caolc::frontend::tree_to_program(&tree, src).expect("lower");
    let program = compile(module, None).expect("compile");

    let mut vm = Vm::new(()).unwrap();
    let err = vm.run(&program).expect_err("expected a runtime error");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("ProcedureNotFound"),
        "expected a native-function-not-found error, got: {msg}"
    );
}
