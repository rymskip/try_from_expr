# try_from_expr

A Rust procedural macro for generating `TryFrom<&syn::Expr>` implementations for
enums. This allows you to parse Rust syntax expressions into strongly-typed enum
values, making it easy to work with configuration DSLs, macro arguments, and
other syntax-based APIs.

## Features

-   🚀 **Automatic Parser Generation** - Derives `TryFrom<&syn::Expr>` for your
    enums
-   📦 **Multiple Variant Types** - Supports unit, tuple, and struct variants
-   🎯 **Smart Type Detection** - Automatically detects wrapper vs leaf enums
-   🔧 **Flexible Parsing** - Works with primitives, collections, and custom
    types
-   💡 **Helpful Error Messages** - Provides clear error messages for parse
    failures
-   ⚡ **Zero Runtime Overhead** - All parsing logic is generated at compile
    time

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
try_from_expr = "0.1"
```

## Quick Start

```rust
use try_from_expr::TryFromExpr;
use syn::Expr;

#[derive(TryFromExpr)]
enum ConfigValue {
    Enabled,
    Threshold(u32),
    Range { min: i32, max: i32 },
}

// Parse from a syn::Expr
let expr: Expr = syn::parse_str("ConfigValue::Threshold(42)").unwrap();
let config = ConfigValue::try_from(&expr).unwrap();
```

## Usage Examples

### Basic Enum with Different Variant Types

```rust
#[derive(TryFromExpr)]
enum Setting {
    // Unit variant
    Default,

    // Tuple variant with single value
    Timeout(u64),

    // Tuple variant with multiple values
    Coordinate(f64, f64),

    // Struct variant
    Config {
        name: String,
        value: i32,
        enabled: bool,
    },
}

// Parse unit variant
let expr = syn::parse_str("Setting::Default").unwrap();
let setting = Setting::try_from(&expr).unwrap();

// Parse tuple variant
let expr = syn::parse_str("Setting::Timeout(5000)").unwrap();
let setting = Setting::try_from(&expr).unwrap();

// Parse struct variant
let expr = syn::parse_str(r#"Setting::Config {
    name: "test",
    value: 42,
    enabled: true
}"#).unwrap();
let setting = Setting::try_from(&expr).unwrap();
```

### Wrapper Enums (Composite Enums)

The macro automatically detects "wrapper" enums that contain other enum types
and generates optimized parsing:

```rust
#[derive(TryFromExpr)]
enum StringConstraint {
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
}

#[derive(TryFromExpr)]
enum NumberConstraint {
    Min(i64),
    Max(i64),
    Range(i64, i64),
}

#[derive(TryFromExpr)]
enum Constraint {
    String(StringConstraint),
    Number(NumberConstraint),
    Required,
}

// The macro detects this is a wrapper and allows parsing nested enums
let expr = syn::parse_str("Constraint::String(StringConstraint::MinLength(10))").unwrap();
let constraint = Constraint::try_from(&expr).unwrap();
```

### Working with Collections

```rust
#[derive(TryFromExpr)]
enum DataType {
    Single(String),
    Multiple(Vec<String>),
    Mapping(HashMap<String, i32>),
}

// Parse Vec
let expr = syn::parse_str(r#"DataType::Multiple(vec!["a", "b", "c"])"#).unwrap();
let data = DataType::try_from(&expr).unwrap();

// Parse HashMap
let expr = syn::parse_str(r#"DataType::Mapping([("key", 42)])"#).unwrap();
let data = DataType::try_from(&expr).unwrap();
```

### Optional Values

```rust
#[derive(TryFromExpr)]
enum OptionalConfig {
    Value(Option<String>),
}

// Explicit Some
let expr = syn::parse_str(r#"OptionalConfig::Value(Some("text"))"#).unwrap();

// Explicit None
let expr = syn::parse_str("OptionalConfig::Value(None)").unwrap();

// Implicit Some (bare value treated as Some)
let expr = syn::parse_str(r#"OptionalConfig::Value("text")"#).unwrap();
```

## Meta Syntax

Every variant also parses from a snake_case form that reads like an attribute
argument, the way `#[serde(rename = "..")]` does. A value inside a meta form
can itself be in path form, as in `regex_literal = Format::Email`. The examples
below use these enums:

```rust
#[derive(TryFromExpr)]
enum Format {
    Email,
    Custom(String),
}

#[derive(TryFromExpr)]
enum StringValidator {
    Trim,
    MaxLength(Option<usize>),
    OneOf(Vec<String>),
    RegexLiteral(Format),
    Length { min: usize, max: Option<usize> },
}

#[derive(TryFromExpr)]
enum NumberValidator {
    Positive,
    Between(i64, i64),
    Labels(HashMap<String, i64>),
}

#[derive(TryFromExpr)]
enum Validator {
    String(StringValidator),
    Number(NumberValidator),
}
```

### Unit variants

A unit variant is its snake_case name. The bare PascalCase name works too.

| Meta form | Parses to                |
| --------- | ------------------------ |
| `trim`    | `StringValidator::Trim`  |
| `Trim`    | `StringValidator::Trim`  |

### Tuple variants

A single-field variant takes `name = value`. An `Option` field accepts `None`,
`Some(..)` or a bare value, and a field holding another enum takes that enum in
either form.

| Meta form                            | Parses to                                       |
| ------------------------------------ | ----------------------------------------------- |
| `max_length = 64`                    | `MaxLength(Some(64))`                           |
| `max_length = Some(64)`              | `MaxLength(Some(64))`                           |
| `max_length = None`                  | `MaxLength(None)`                               |
| `one_of = ["draft", "published"]`    | `OneOf(vec!["draft".into(), "published".into()])` |
| `one_of = vec!["draft"]`             | `OneOf(vec!["draft".into()])`                   |
| `regex_literal = format(email)`      | `RegexLiteral(Format::Email)`                   |
| `regex_literal = Format::Email`      | `RegexLiteral(Format::Email)`                   |
| `regex_literal = format(custom = "^a")` | `RegexLiteral(Format::Custom("^a".into()))`  |

A variant with several fields takes them as a tuple, and a map takes an array
of key and value pairs.

| Meta form                              | Parses to                                  |
| -------------------------------------- | ------------------------------------------ |
| `between = (1, 10)`                    | `NumberValidator::Between(1, 10)`          |
| `between = (-5, 5)`                    | `NumberValidator::Between(-5, 5)`          |
| `labels = [("low", 0), ("high", 100)]` | `NumberValidator::Labels({"low": 0, "high": 100})` |

### Struct variants

A struct variant is called with `field = value` arguments, in any order. A
field of type `Option` can be left out.

| Meta form                       | Parses to                              |
| ------------------------------- | -------------------------------------- |
| `length(min = 1, max = 64)`     | `Length { min: 1, max: Some(64) }`     |
| `length(max = 64, min = 1)`     | `Length { min: 1, max: Some(64) }`     |
| `length(min = 1)`               | `Length { min: 1, max: None }`         |

### Naming the enum

Any meta form can be wrapped in its enum's snake_case name. On the enum itself
this changes nothing, but in a wrapper it picks the child explicitly.

| Meta form                                | Parses to                                     |
| ---------------------------------------- | --------------------------------------------- |
| `string_validator(trim)`                 | `StringValidator::Trim`                       |
| `string_validator(length(min = 1))`      | `StringValidator::Length { min: 1, max: None }` |
| `number_validator(between = (1, 10))`    | `NumberValidator::Between(1, 10)`             |

### Wrapper enums

A wrapper accepts its children's forms. A bare name goes to the child that
accepts it (see [Children Sharing a Name](#children-sharing-a-name)), a named
child goes to that child, and the wrapper's own variants take `name = value`
like any tuple variant.

| Meta form given to `Validator`       | Parses to                                          |
| ------------------------------------ | -------------------------------------------------- |
| `trim`                               | `Validator::String(StringValidator::Trim)`         |
| `between = (1, 10)`                  | `Validator::Number(NumberValidator::Between(1, 10))` |
| `string_validator(max_length = 64)`  | `Validator::String(StringValidator::MaxLength(Some(64)))` |
| `number = positive`                  | `Validator::Number(NumberValidator::Positive)`     |
| `validator(string = trim)`           | `Validator::String(StringValidator::Trim)`         |
| `StringValidator::Trim`              | `Validator::String(StringValidator::Trim)`         |

### Keywords

A name that is a Rust keyword takes the raw identifier prefix, so a variant
`Type(String)` is written `r#type = "text"` and a variant `Ref { r#type: u8 }`
is written `r#ref(r#type = 3)`.

### Parsing an attribute

The meta forms are ordinary Rust expressions, so an attribute's arguments parse
as a comma-separated list of `syn::Expr`:

```rust
use syn::{Attribute, Expr, Token, punctuated::Punctuated};

let attr: Attribute = syn::parse_quote!(#[validators(
    trim,
    length(min = 1, max = 64),
    string_validator(regex_literal = format(custom = r"^/(?:[^/\\].*)?$")),
)]);

let validators = attr
    .parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?
    .iter()
    .map(Validator::try_from)
    .collect::<syn::Result<Vec<_>>>()?;
```

The equivalent path forms parse to the same values:

```rust
#[validators(
    StringValidator::Trim,
    StringValidator::Length { min: 1, max: Some(64) },
    StringValidator::RegexLiteral(Format::Custom(r"^/(?:[^/\\].*)?$")),
)]
```

### Mistakes

A form that names nothing reports the names that would have worked:

```text
length(min = 1, mx = 64)
    Unknown field 'mx' for variant 'Length'. Valid fields: min, max

string_validator(trimm)
    Unknown unit variant 'trimm' for enum 'StringValidator'.
    Valid options: Trim; in meta form: trim

between = 1
    Variant 'Between' expects a tuple of 2 values
```

### A struct variant named after its enum

When a struct variant shares its enum's snake_case name, `name(..)` is that
variant when every argument is one of its fields, and the enum wrapper
otherwise. A field of that variant may not share a meta name with a tuple
variant, since `name(field = ..)` would then fit both.

## Children Sharing a Name

Every derived enum publishes the names it accepts bare, through the
`try_from_expr::meta_names::MetaNames` trait. A wrapper uses them to send a bare
name such as `trim` straight to the one child that accepts it. Names only clash
within a shape: `required`, `required = ..` and `required(..)` are distinct.

When two children accept the same bare name, using it bare is an error that
asks for the qualified form:

```text
Ambiguous meta name `required` for enum 'Rule': accepted by TextRule, NumberRule.
Qualify it with `text_rule(..)` or `number_rule(..)`
```

To rule out overlaps altogether, mark the wrapper `all_unique`. Any shared name,
including one inside a nested wrapper, then fails to compile:

```rust
#[derive(TryFromExpr)]
#[try_from_expr(all_unique)]
enum Rule {
    Text(TextRule),
    Number(NumberRule),
}
```

A hand-written child type implements `MetaNames` itself, listing the bare names
its `TryFrom<&syn::Expr>` accepts:

```rust
impl MetaNames for Flag {
    const META_NAMES: MetaNameSet = MetaNameSet {
        path_names: &["flag"],
        ..MetaNameSet::empty("Flag")
    };
}
```

## Force Mode Selection

By default, the macro automatically detects whether your enum is a wrapper or
leaf enum. You can override this:

```rust
#[derive(TryFromExpr)]
#[try_from_expr(wrapper)]  // Force wrapper mode
enum ForceWrapper {
    Variant1(CustomType),
    Variant2(AnotherType),
}

#[derive(TryFromExpr)]
#[try_from_expr(leaf)]  // Force leaf mode
enum ForceLeaf {
    Simple,
    Complex(String),
}
```

## How It Works

The macro analyzes your enum at compile time and generates a
`TryFrom<&syn::Expr>` implementation that:

1. **Unwraps** any parentheses or group expressions
2. **Matches** the expression type (path, call, struct, or a meta form)
3. **Parses** the variant name and validates it belongs to your enum
4. **Extracts** and parses any parameters or fields
5. **Constructs** the appropriate enum variant
6. **Returns** helpful error messages for any parsing failures

## Supported Types

The macro has built-in support for:

-   **Primitives**: `bool`, `char`, `String`, all integer types, `f32`, `f64`
-   **Collections**: `Vec<T>`, `HashMap<K, V>`, `BTreeMap<K, V>`, `Option<T>`
-   **Special**: `OrderedFloat<T>` from the `ordered-float` crate
-   **Custom Types**: Any type that implements `TryFrom<&syn::Expr>`, plus
    `MetaNames` when a wrapper enum holds it

## Error Handling

The macro provides detailed error messages:

```rust
// Unknown variant
"Unknown unit variant 'Invalid' for enum 'Setting'"

// Wrong number of arguments
"Variant 'Coordinate' expects 2 argument(s), but 3 were provided"

// Type parsing failure
"Failed to parse argument 1: Expected an integer literal"

// Missing required field
"Missing required field 'name' for variant 'Config'"

// Unknown meta variant
"Unknown unit variant 'trimm' for enum 'StringValidator'. Valid options: Trim; in meta form: trim"
```

## Project Structure

This workspace contains two crates:

-   `try_from_expr` - The main library crate that users import
-   `try_from_expr_derive` - The procedural macro implementation

## License

This project is licensed under the MIT license.

## Contributing

Please feel free to submit a Pull Request.
