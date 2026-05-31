# What Is an AST?

The lexer handed us a flat list of tokens, and a flat list has a problem it cannot solve on its
own. Take `2 + 3 * 4`. That's five tokens in a row — `2`, `+`, `3`, `*`, `4` — and if you stare
at them as a sequence, there's no way to tell which operation happens first. Do we add `2 + 3` and
then multiply by `4`? Or multiply `3 * 4` and then add `2`? You know the answer is the second one,
because you learned operator precedence in school. But the token list doesn't know that. Tokens
have no precedence. They're just beads on a string.

The answer is to stop thinking of the program as a line and start thinking of it as a tree. Here
is `2 + 3 * 4` as a flat token list:

```
IntLiteral(2)  Plus  IntLiteral(3)  Star  IntLiteral(4)
```

And here is the same expression as a tree:

```
    BinaryOp(+)
   /           \
IntLit(2)   BinaryOp(*)
            /          \
        IntLit(3)   IntLit(4)
```

Look at what the tree did. The multiplication isn't sitting *next to* the addition anymore — it's
*underneath* it, a subtree nested inside the right branch. And because it's deeper in the tree,
it gets evaluated first: you can't compute the `+` node until you've computed both of its children,
and one of its children is the `*`. Precedence stopped being a rule you have to remember and became
the literal shape of the data. There is no ambiguity left to resolve, because the structure already
resolved it.

That tree is called an Abstract Syntax Tree, and building it is the parser's entire job. The next
few chapters are about how it pulls that off.

## From flat to structured

The lexer gives us tokens — a flat sequence. The problem is that code is not flat. It has
structure. Consider `2 + 3 * 4`.

As tokens:
```
IntLiteral(2)  Plus  IntLiteral(3)  Star  IntLiteral(4)
```

That's five tokens in a row. But which two get multiplied? Which two get added? The token
list is ambiguous. The meaning depends on operator precedence — and tokens have no precedence.

The AST resolves this. The same expression as a tree:

```
    BinaryOp(+)
   /           \
IntLit(2)   BinaryOp(*)
            /          \
        IntLit(3)   IntLit(4)
```

Now it's unambiguous. To evaluate this tree: evaluate the right subtree first (3 * 4 = 12),
then use its result in the parent (2 + 12 = 14). The structure encodes precedence.

## The AST is the program in data-structure form

An AST (Abstract Syntax Tree) is a tree of nodes where each node represents a construct
in the source code: a function definition, a variable binding, a binary operation, a loop,
a struct literal. The root node represents the entire program.

"Abstract" means details that don't affect meaning are stripped out — semicolons,
parentheses used just for grouping, whitespace. The tree captures *structure*, not the
original text.

After parsing, the token list is no longer needed. Every downstream stage — type checker,
IR gen, codegen — works on the AST.

## What Oxy's AST looks like

Oxy's AST is defined in `crates/oxy-core/src/ast/mod.rs`. The top-level type is `Program`:

```rust
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}
```

A `Program` is a list of `Item`s — top-level declarations:

```rust
pub enum Item {
    Function(FnDef),
    Struct(StructDef),
    Enum(EnumDef),
    Impl(ImplBlock),
    Trait(TraitDef),
    ImplTrait(ImplTraitBlock),
    Module(ModuleDef),
    Use(UseDef),
    TypeAlias { name: String, target: TypeAnnotation, span: Span },
    Const { name: String, type_ann: Option<TypeAnnotation>, value: Expr, ... },
}
```

A `FnDef` has a name, parameters, a return type, and a body:

```rust
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub span: Span,
    // ...
}
```

A `Block` is a list of statements:

```rust
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}
```

A `Stmt` is one statement — a `let` binding, a loop, a return, or an expression:

```rust
pub enum Stmt {
    Let { name: String, mutable: bool, type_ann: Option<TypeAnnotation>, value: Option<Expr>, span: Span },
    Return { value: Option<Expr>, span: Span },
    While { condition: Box<Expr>, body: Block, span: Span, .. },
    Expr { expr: Expr, has_semicolon: bool },
    // ...
}
```

And `Expr` is an expression — a literal, an operation, a call, a match:

```rust
pub enum Expr {
    IntLiteral(i64, IntegerSuffix, Span),
    BinaryOp { left: Box<Expr>, op: BinOp, right: Box<Expr>, span: Span },
    Call { callee: Box<Expr>, args: Vec<Expr>, span: Span, .. },
    If { condition: Box<Expr>, then_block: Block, else_block: Option<Block>, span: Span },
    // ... 30+ more variants
}
```

## Why `Box<Expr>`?

You will see `Box<Expr>` throughout the AST — in `BinaryOp`, `If`, `While`, and many others.
This is necessary because Rust requires all types to have a known size at compile time.

An `Expr` can contain another `Expr` (the left and right sides of a binary operation are
`Expr`s). If `Expr` directly contained `Expr`, the size of `Expr` would depend on itself —
infinite recursion at the type level. Rust rejects this.

`Box<Expr>` is a pointer to a heap-allocated `Expr`. Its size is always the same (one pointer
width), regardless of what `Expr` it points to. This breaks the recursive size dependency.

We explain `Box` in depth in the Rust concepts chapter of this part.

## AST nodes carry spans

Every node in the AST carries a `Span` — the same span type we saw in the lexer. This means
every piece of the AST can say where in the source it came from.

When the type checker reports an error like "expected int, found String at line 7, column 3",
it gets that line and column from the span on the relevant AST node. The span is the thread
that connects source text to error messages throughout the entire pipeline.

## A complete example: tracing `let x = 2 + 3;`

This source text:
```
let x = 2 + 3;
```

Becomes this AST:
```
Stmt::Let {
    name: "x",
    mutable: false,
    type_ann: None,
    value: Some(
        Expr::BinaryOp {
            left: Box(Expr::IntLiteral(2, None, span(1:9))),
            op: BinOp::Add,
            right: Box(Expr::IntLiteral(3, None, span(1:13))),
            span: span(1:9-1:13),
        }
    ),
    span: span(1:1-1:14),
}
```

The `let` keyword, the `=`, the `;` — gone. The structure remains. The spans remain.
Everything downstream needs to know is captured in this tree.
