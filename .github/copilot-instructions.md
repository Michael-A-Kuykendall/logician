# Sorcery Architecture Primer

This project follows the **Sorcery** design doctrine. This is an absolute, architecturally locked system. It is not a sketch pad; it is a precise blueprint language for "hydrating" and "dehydrating" intent.

## Core Philosophy

1.  **Intent is Architecturally Locked**: Logic acts as a "spell" that binds **Intent** (the "why") to **Evidence** (the tests).
2.  **Context Asymmetry**:
    -   **Casting (Dehydration)**: A high-context architect compresses distinct reasoning into a concise Spell.
    -   **Invocation (Rehydration)**: A high-speed agent/developer expands that Spell into code and tests.
    -   *Crucial*: The Invoker (Creator of Code) never invents intent. If it's not in the Spell, it doesn't exist.
3.  **No Logic Without Proof**: Every architectural claim (`$`) must be backed by a test or a structural guarantee.

## The Glyph Notation

We use **Glyph**, a symbolic notation to define Spells.

| Symbol | Meaning | Function |
|:---:|---|---|
| `#` | **Spell Name** | Defines the atomic unit of capability. |
| `^` | **Intent** | **Mandatory.** The "why" that survives strict tradeoffs. |
| `@` | **Component** | The entity (function, class, module) being defined. |
| `:` | **Contract** | `Input -> Output`. The shape of the data flow. |
| `>` | **Dependency** | `@A > @B` means A depends on B. Defines implementation order. |
| `$` | **Obligation** | The core constraints. See types below. |
| `~` | **Assumption** | Runtime truths we assume but don't prove (e.g. `~ valid_utf8`). |
| `?` | **Open Question** | A blocker. A spell cannot be cast until all `?` are resolved. |

### Obligation Types (`$`)

*   **`$ require: fn name`**: The artifact must exist.
*   **`$ forbid: concept`**: Negative space architecture. The system *must not* do this (e.g. `$ forbid: network`, `$ forbid: wildcards`).
*   **`$ prove: behavior -> test: name`**: The binding rule. Every logical claim must map to a specific executable test case.

## Working with Sorcery

When working in this workspace, follow these rules:

1.  **Cast First**: Before writing code, we must "Cast" the Spell. Define the logic in Glyph.
2.  **Seal the Spell**: Ensure there are no `?` (open questions).
3.  **Invoke explicitly**:
    -   Implement the structure defined in `@` and `:`.
    -   Implement the dependency chain defined by `>`.
    -   **Bind Tests**: For every `$ prove:`, create a test file/case that explicitly verifies that specific claim.
    -   **Enforce Forbids**: Ensure architecture does not violate `$ forbid` rules (often via linter rules or strict typing).

## Example Spell

```glyph
#Spell: Tokenize
^ Intent: produce stable tokens for deterministic inference

@Tokenizer
  : utf8 -> tokens
  $ require: fn tokenize
  $ forbid: network
  $ prove: deterministic -> test: det
  $ prove: empty_input_safe -> test: empty
  ~ valid_utf8
```
