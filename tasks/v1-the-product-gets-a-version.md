---
tags: my, cyberia.my, release, task
alias: v1 the product gets a version, v1
kind: task
status: doing
owner: the executor's sessions
serves: my cyberia · the door says what it serves
lane: wide
date: 2026-11-21
---
# v1 · the product gets a version

## claim

The owner, 2026-11-21 16:56, a signed command to `product/my`: *bump*
(`apparatus-commands/commands/2026-11-21/1655-dec70661.md`).

The product had nothing to bump. `Cargo.toml` carried `0.1.0` from the first commit of 2026-08-10
through 167 commits — the globe, the land market, MY PLOTS, the desk, the library — and the
repository has no tag, no release and no changelog. The cockpit reads a product's version from the
door it serves and compares it with the latest release
(`apparatus/scripts/cockpit.py` § `served`); for `my` it reads `release: null`, `version: null`,
`commits_since: null`. The products screen shows the product's size and its three open pull
requests, and a blank where every other product states what is live.

## what is written

| where | what |
|---|---|
| `Cargo.toml` | `version = "0.2.0"` — the first bump of a live product, a minor one: the console of `0.1.0` became the land console |
| `CHANGELOG.md` | the product's history in two sections: `0.2.0` grouped by what it does, `0.1.0` as the console that was never tagged |
| `index.html` | `<meta name="version" content="0.2.0" />` — the door states what it serves in its page source, where the cockpit's `served` scans for a three-number version and matches it against the latest release |
| `src/nav.rs` | `VERSION` from `CARGO_PKG_VERSION`, shown as one line under the parked surfaces on `/menu` |
| `tests/version.rs` | the three move together: the meta is the crate's version, the changelog opens with it, and a version is three numbers |

No sibling pins: nothing pins `cyberia-my`, and it pins `hemera` and `mudra` by path, not by version.

## what is not done here

- the tag `v0.2.0` and the GitHub release are the owner's hand (`~/cyber/AGENTS.md` § release train):
  the plan goes back to the apparatus with this command's answer, signed there, run by the executor.
- the bin builds with 19 dead-code warnings that predate this change (`market_buy`, `advance_deal`,
  `STEPS`, …). They are a sweep of their own, not a version bump.

## steps

| # | step | state |
|---|---|---|
| 1 | the version, the changelog, the door's meta, the line on `/menu`, the test | done 2026-11-21 |
| 2 | the combine's verdict on the pull request | open |
| 3 | the tag and the release, by the owner's signed plan | open |
