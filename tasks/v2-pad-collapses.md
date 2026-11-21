---
tags: my, cyberia.my, task
alias: v2 the pad collapses
kind: task
status: doing
owner: the executor's sessions
serves: my cyberia · the globe's navigation pad
lane: contained
date: 2026-11-21
---
# v2 · the pad collapses

## claim

The owner's commander (joyshop.eth), a signed command to `apparatus`
(`apparatus-commands/commands/2026-11-17/1734-11c19232.md`): the control panel on the left of the
globe should be possible to hide (compress) and be pinned to the left corner of the screen.

## what is written

| where | what |
|---|---|
| `src/pad.rs` | the pad's own module: every camera key, the toggle first, and `pad_hidden` kept in `localStorage` (`cyberia.pad_hidden.v1`) |
| `src/console.rs` | the pad is one line now, `<NavPad …/>` — the file is over the house's 500-line cap and must not grow |
| `style.css` | the pad is pinned at the top-left corner of the map; hidden, only the toggle chip stays |
| `tests/pad_collapse.rs` | the key, the hidden class and the corner agree across the module, this page and the style |

## done when

The pad collapses to one chip and expands back, in the top-left corner, and the choice survives a reload.

## not verified

The agent's host has no Rust toolchain: the change was not compiled. CI and the pull request's preview are the first build.
The round trip through `localStorage` is not exercised by a test: the crate carries no wasm test harness and the house
bans node, so the browser's side of the toggle is first seen on the preview. `tests/pad_collapse.rs` holds what a native
test can hold — that the key, the class and the corner are named the same in every text that has to agree.
