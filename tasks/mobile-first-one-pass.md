---
tags: my, cyberia.my, mobile, task
alias: mobile first, one pass
kind: task
status: doing
owner: the executor's sessions
serves: my cyberia · a visitor on a phone
lane: wide
date: 2026-11-21
---
# mobile first, one pass

## claim

The owner, 2026-11-20, a signed command to `product/my`: *mobile version sucks; run one complete
round of an astonishing mobile-first experience*
(`apparatus-commands/commands/2026-11-20/1158-181dec07.md`). Done when a visitor on a phone sees a
fast, polished, mobile-first layout.

## what is written

One change, one stylesheet block (`style.css`, last section, "mobile first, one pass"):

| where | what |
|---|---|
| header | the four places become one strip under the logo, 44 px targets, scrolling sideways instead of wrapping under the player; safe-area insets |
| targets | buttons, rows, the pad, close buttons reach 44 px |
| forms | inputs 16 px (no zoom on focus), full width, 44 px high |
| overflow | tables and code scroll inside themselves; images never wider than the screen |
| touch | pressed state where only `:hover` existed; no rubber band; tap highlight in the house green |
| small phones | 360 px: single-column steps and facts |
| motion | `prefers-reduced-motion` honoured |
| `tests/mobile.rs` | the viewport, the braces, the 16 px, the 44 px, the strip, the reduced motion |

## what is not done here

- the build host has no cargo, trunk or browser: the block is checked by text (braces balanced) and
  not yet seen on a device. The review and the preview (`<N>.patch.cyberia.my`) are where it is seen
  and tried; a finding there is a fix on this branch.
- the map's own gestures (`globe.rs`) are untouched; this pass is the shell around them.

## steps

| # | step | state |
|---|---|---|
| 1 | the stylesheet block, the test, this page | done 2026-11-21 |
| 2 | the preview placed, the combine's review | open |
