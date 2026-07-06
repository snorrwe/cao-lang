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

Handles: `fn NAME(params) { ... }` with parameters, `return EXPR;`, `global NAME = EXPR;` / local `NAME = EXPR;` (`global_var_statement` / `local_var_statement`), call expressions (including method-call-style `obj.method(x)` via `_callee = choice(identifier, postfix_expression)`), binary expressions (`<= < == != + - * /` with precedence tiers via `prec.left`), and `if COND { EXPR }` with an optional `else { EXPR }` as an expression (`if_expression` / `expression_block` — maps to `IfTrue` when `else` is omitted, `IfElse` when present).

Also handles: integer/float literals (both signed, e.g. `-5`, `-1.5` — the tree-sitter lexer is parser-state-aware so this doesn't collide with binary `-`), string literals with escapes (`\" \\ \n \r \t \0 \u{...}`), dotted property access and index access as a shared `postfix_expression` (`a.b.c`, `a[i]`, chainable and mixable, `prec(4)` — binds tighter than all binary ops), property/index assignment (`set_property_statement`: `a.b = x;`, `a[i] = x;`), bare expressions as statements (`expression_statement`: `f(x);`), array literals (`[1, 2, 3]`), table literals (`table_expression`/`table_field`, `#{a: 1, "b-c": 2}` — a dedicated `#{` token avoids colliding with `block`/`expression_block`'s bare `{`), `while COND { ... }`, `repeat N [as VAR] { ... }`, `foreach [i,][k,]v in EXPR { ... }` (`foreach_bindings` — positional prefix of `i`/`k`/`v` matching `ForEach`'s field order), JS-style arrow closures with always-parenthesized params and either an expression or block body (`closure_expression`: `(a, b) => a + b`, `(a, b) => { ... }`), `import a.b.c;` (`import_statement`/`dotted_path`, no alias clause — matches the compiler's `rsplit_once('.')` binding of the last segment), and `mod NAME { ... }` submodules (`module_definition`, nests `_definition` recursively).

`if_expression`'s branches are still single expressions (`expression_block`), not statement lists — a bare `if { ... }` used as a standalone statement (not wrapped in `return`) isn't supported. Verified via `test/corpus/*.txt` (no `.caol` fixture files are kept in the repo — the one that existed, `fibonacci_program_recursive.caol`, was deleted once ported into `test/corpus/functions.txt`; corpus tests are the source of truth).

Also handles: boolean/logical operators `&& || ^` (binary, precedence tiers 3/2/1 respectively — looser than comparisons, tighter than nothing) and prefix `!` (`unary_expression`, precedence tier 7 — binds tighter than every binary operator but looser than `postfix_expression`, which sits at tier 8), the `nil` literal (`nil_literal`), a bare `abort` keyword expression (`abort_expression`), `len(x)` (`len_expression`), `push(table, value)` / `pop(table)` for table mutation (`push_expression`/`pop_expression`, mapping to `CardBody::AppendTable`/`PopTable` — note `push`'s argument order is `(table, value)` while `AppendTable`'s doc comment specifies storage order `[Value, Table]`, so a frontend needs to swap them), and `&name` / `&mod.path` function-pointer-value references (`function_ref_expression`, reusing `dotted_path`, mapping to `CardBody::Function`/`NativeFunction` via the same known/unknown-declared-function check used for `call_expression` callees). `nil`/`abort`/`len`/`push`/`pop` are now reserved words (can no longer be used as identifiers), same mechanism as existing keywords like `fn`/`if`/`while`.

Explicitly out of scope (no grammar rule, flagged in code comments where relevant): `DynamicCall` has no dedicated syntax of its own — once a variable holds a function-pointer value (via `&name`), calling it goes through the *existing* `call_expression` syntax (`f(...)` where `f` is an identifier), but the frontend lowering pass doesn't yet distinguish "callee is a local variable holding a function value" from "callee is a declared function/native name" (it only ever emits `Call`/`CallNative`, never `DynamicCall`) — that's a frontend semantic-analysis change, not a grammar gap.

## Workflow

- Regenerate parser after grammar changes: `just build`
- Run corpus tests: `just test` (runs `tree-sitter test` against `test/corpus/*.txt`)
- Corpus test format: `====name====` / source / `---` / expected S-expression (field names shown as `name: (node)`). Get the expected tree by running `tree-sitter parse <file>` and stripping the `[row, col] - [row, col]` position ranges.
- `test/*.caol` are the underlying source fixtures corpus cases are built from; add new syntax coverage by extending both a `.caol` fixture and a matching `test/corpus/*.txt` case alongside grammar rules.
