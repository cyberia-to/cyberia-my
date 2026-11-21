---
tags: cyberia, project, cyberia-my, land
alias: funnels, the ways in, become an agent, buy a franchise, add your land, found a city
crystal-type: plan
crystal-domain: cyberia
status: proposed
date: 2026-11-21
owner: master
serves: land
lane: wide
---

# the ways in: land, agents, franchises, cities

the owner, 2026-11-21: the globe's first panel must close the app's functions; the funnels are thought through end to end, the agent's above all; and a funnel to buy a franchise of a country or a territory is needed. it continues [[listings]]: the same gate, the same signed acts.

| field | value |
|---|---|
| delivery | from the globe's first panel a person reaches everything the app does: my plots, sell or sublease, add your land, become an agent, buy a franchise, found a city, and the desk for the wallets that may sit at it |
| stage | proposed · green on a bench with throwaway keys (2026-11-21): the four funnels signed, answered on the desk, the agent's desk and referral counted |

## one act, four things

`apply` is one act with `what: land | agent | franchise | city`. anyone signs it; an admin answers with `approve` (`application: <id>`) or `reject` (`target: <id>`); the applicant takes it back with `withdraw`. a new application of the same thing takes the place of the waiting one. the contract is the header of `gate/acts.py`; the tests are `ApplyTest` in `gate/acts_test.py`.

| what | page | what it carries | what approval does |
|---|---|---|---|
| land | `/land` | place, point, ares, the paper, sale or lease | shows on the public board; its flats are then drawn and bound by the valley |
| agent | `/agent` | territory, languages, company, the code accepted | gives the role `agent`, a profile when none was signed, a place in the directory |
| franchise | `/franchise` | territory, level state, region or district, the state's code, the company | shows on the public board |
| city | `/found` | name, point, territory | shows on the public board |

## the agent, end to end

1. `/agent`: wallet, where and in which languages they sell, the code, one signature.
2. the desk, REQUESTS: the valley accredits or rejects.
3. accredited, `/agent` is the agent's desk: a link for holders (`/sell?agent=0x…` opens the mandate step with the agent named), a link for everyone else (`/?ref=0x…`), the mandates held, what the people they sent applied for.
4. a holder at `/sell` picks the agent from the directory of accredited agents, or names any address, and signs the mandate.
5. the agent lists the flat at `/sell`; the valley approves; the map shows it with the badge.

a referral is the field `ref` of an application: the address kept from `?ref=` in the browser. the agent sees what was applied for and how it stands, never who.

## the code and the terms

the texts a person accepts are in `src/apply.rs` (`KINDS`). the agent's code states only what the acts enforce: a mandate for every offer, the commission written in the mandate, the valley's approval, one signature to end it, the badge given and taken by the valley.

## open

| question | where it is decided |
|---|---|
| the franchise's fee, share and term; whether a territory is exclusive | the page says they are agreed per territory after the application; no number is written anywhere |
| whether the valley takes a part of an agent's commission | not stated on the page |
| a referral's reward | counted, not paid |
| drawing an accepted land as flats | by hand, through the map layer and `scripts/sync-titles.py` |
