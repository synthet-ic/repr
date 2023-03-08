**TODO**

- `ignore`, or lookahead/behind
- Derivatives

**`Map`**

- The functional equivalent to grouping and capturing. 

**`Zero`**

- `Zero` acts as both an additive and multiplicative unit?

**`Div`** (quotients)

- Case insensitivity

**`Rev`** (?)

**`Rem`** (partial match)

**`Sh`** (shift position?)

**Comparisons**

| regex | linear logic | repr | op |
| - | - | - | - |
| ∅ | | `Zero` | |
| `a` | `a` | `One(a)` | |
| ε (empty) | | `One('')` |
| `ab`/`a` · `b` (concatenation) | `a` & `b` (additive conjunction/with) | `Mul(a, b)` | `*` |
| `a\|b` (alternation) | `a` ⊕ `b` (additive disjuction/plus) | `Or(a, b)` | `\|` |
| `a*` (kleen star) | `!a` (of course) | `Exp(a)` |
| `a*?` (non greedy) | `?a` (why not) | `Exp(a)` |
| `a?` () | `a` | `Or(Zero, a)` |
| `a{n,m}` (repetition) | `a` | `Or(Mul(a, Mul(a, ..)), Or(..))` |
| `[a-z]` (class) | | `Seq(a, z)` | `..` |
| `[^a-z]` (negation) | | `Not(a)` | `!` |
| `a`<sup>†</sup> (reverse) | | `Rev(a)` | `-` |
| `a` / `b` (right quotient) | | `Div(a, b)` | `/` |
| `a` \ `b` (left quotient) | | `Div(a, b)` | `/` |
| RegexSet | `a` ⅋ `b` (multiplicative disjunction/par) | `Add(a, b)` | `+` |
| `a` ∩ `b` (intersection) | `a` ⊗ `b` (multiplicative conjunction/times) | `And(a, b)` | `&` |
| `a(?=b)` (positive lookahead) | | `And(a, b)` | |
| `a(?!b)` (negative lookahead) | | `And(a, Not(b))` | |
| `(?<=a)b` (positive lookbehind) | | `And(a, b)` | |
| `(?<!a)b` (negative lookbehind) | | `And(a, b)` | |

**Laws**

| formula | title |
| - | - |
| Or(a, Or(b, c)) = Or(Or(a, b), c) | Or-associativity |
| | |
| Or(a, a) = a | Or-idempotence |
| Or(a, Zero) = Or(Zero, a) = a | Zero, Or-unit |
| Mul(a, One('')) = Mul(One(''), a) = a | One, Mul-unit |
| Mul(a, Zero) = Mul(Zero, a) = Zero | Zero, Mul-zero |
| Mul(Or(a, b), c) = Or(Mul(a, c), Mul(b, c)) | right distributivity |
| Mul(a, Or(b, c)) = Or(Mul(a, b), Mul(a, c)) | left distributivity |
| Rev(One(a)) = One(a) | |
| Rev(Mul(a, b)) = Mul(Rev(b), Rev(a)) | |
| Mul(One(a), One(b)) = One(ab) | |

**Flags (TODO)**
- `i`, CaseInsensitive
- `m`, MultiLine
- `s`, DotMatchesNewLine
- `U`, SwapGreed
- `u`, Unicode
- `x`, IgnoreWhitespace
```

- Recouse non-greedy pattern to _
