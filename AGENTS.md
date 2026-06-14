# Repository Instructions

## Rust Module Layout

Prefer the modern Rust module layout:

- Use `foo.rs` as the module entry file.
- Put child modules under `foo/`.
- Do not add new `foo/mod.rs` module entry files.

For example, use `src/sections.rs` plus `src/sections/*.rs`, not
`src/sections/mod.rs`.
