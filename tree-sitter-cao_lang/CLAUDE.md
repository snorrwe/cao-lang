# tree-sitter-cao_lang

Tree-sitter grammar for CaoLang. Goal: parse `.caol` source text into a tree whose shape maps cleanly onto the `Card`/`CardBody` AST in `../cao-lang/src/compiler/` (`Compiler::compile` consumes that AST and emits bytecode — see `../cao-lang/src/compiler.rs`). This repo does not touch the compiler; it only needs to produce a CST an external step can walk into `CaoProgram`.

## Layout

- `grammar.js` — grammar rules (WIP, only a toy subset done so far)
- `test/*.caol` — sample programs used as parse fixtures
- `src/` — generated parser (do not hand-edit; regenerate with `tree-sitter generate`)
- bindings for node/python/rust are scaffolded but not the focus

## Target AST (source of truth: `../cao-lang/src/compiler/`)

- `Module` (`module.rs`) — root program: `functions: Vec<(name, Function)>`, `submodules`, `imports`. Must have a `main` function.
- `Function` (`function.rs`) — `arguments: Vec<VarName>`, `cards: Vec<Card>`.
- `Card` (`card.rs`) — `{ id, body: CardBody }`. `CardBody` variants to eventually cover:
  - arithmetic/logic (binary, 2 children): `Add Sub Mul Div Less LessOrEq Equals NotEquals And Or Xor Get GetProperty AppendTable`
  - unary (1 child): `Not Return Len PopTable`
  - control flow: `IfTrue` (cond, then), `IfElse` (cond, then, else), `While` ([cond, body]), `Repeat` (n, optional loop-var, body), `ForEach` (i/k/v optional bindings, iterable, body)
  - vars: `ReadVar(name)`, `SetVar{name, value}`, `SetGlobalVar{name, value}` (source syntax uses `global x = ...;`, see fixture)
  - calls: `Call{function_name, args}` (static), `DynamicCall{function, args}`, `CallNative`, `Function(name)` (function pointer value), `NativeFunction(name)`, `Closure` (inline fn value capturing upvalues)
  - literals: `ScalarInt(i64)`, `ScalarFloat(f64)`, `StringLiteral(String)`, `ScalarNil`, `CreateTable`, `Array(Vec<Card>)`
  - misc: `SetProperty[value, table, key]`, `CompositeCard{ty, cards}` (opaque group, not a parse target), `Abort`
- Property/dotted access (`a.b.c`) and imports use `.`-joined name strings resolved by the compiler at namespace/import lookup time (`resolve_function`, `read_var_card` in `compiler.rs`) — the grammar just needs to emit the dotted identifier text, not resolve it.

## Known concrete syntax (from `test/fibonacci_program_recursive.caol`)

```
fn main(N) {
    global result = fib(N);
}

fn fib(n) {
    return if n <= 2 { n } else { fib_again(n) };
}

fn fib_again(n) {
    return fib(n-1) + fib(n-2);
}
```

Notable: `if`/`else` is an *expression* (used directly in `return`), `global NAME = EXPR;` maps to `SetGlobalVar`, plain `NAME = EXPR;` (not yet in fixture) should map to `SetVar`, function calls are `name(args...)`.

## Current grammar.js state

Handles: `fn NAME(params) { ... }` with parameters, `return EXPR;`, `global NAME = EXPR;` (`global_var_statement`), call expressions, binary expressions (`<= < == != + - * /` with precedence tiers via `prec.left`), and `if COND { EXPR } else { EXPR }` as an expression (`if_expression` / `expression_block`). Confirmed against `test/fibonacci_program_recursive.caol` (see `test/corpus/functions.txt`). Still unimplemented — marked with `// TODO` comments in the file: local `NAME = expr;` (`SetVar`, no `global`), `while`/`repeat`/`foreach`, arrays, string/float literals, dotted property access, closures, imports/submodules.

## Workflow

- Regenerate parser after grammar changes: `just build`
- Run corpus tests: `just test` (runs `tree-sitter test` against `test/corpus/*.txt`)
- Corpus test format: `====name====` / source / `---` / expected S-expression (field names shown as `name: (node)`). Get the expected tree by running `tree-sitter parse <file>` and stripping the `[row, col] - [row, col]` position ranges.
- `test/*.caol` are the underlying source fixtures corpus cases are built from; add new syntax coverage by extending both a `.caol` fixture and a matching `test/corpus/*.txt` case alongside grammar rules.
