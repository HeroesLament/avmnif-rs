# avmnif-rs

**A tiny, no-std-friendly toolkit for writing AtomVM NIFs in safe Rust.**

AtomVM embeds the Erlang/Elixir VM on micro-controllers.  `avmnif-rs`
lets you expose native Rust functions (“NIFs”) to that VM without touching
unsafe C boilerplate.

```text
$ cargo add avmnif-rs
