---
tags: cyberia, project, cyberia-my, land
alias: listings, the land market, holders' offers, resale, sublease, agents
crystal-type: plan
crystal-domain: cyberia
status: proposed
date: 2026-11-20
owner: master
serves: land
lane: wide
---

# the land market: holders, agents, offers

the owner, 2026-11-20: an admin interface and a funnel for the owners who want to place their plots on the map for sale or lease. three answers shaped it: only flats inside Cyber Valley; every interaction happens the way it happens on the apparatus, with web3 and commits; and a plot can be sold by a representative, an agent — a central part of the system.

| field | value |
|---|---|
| delivery | a holder, or the holder's agent, puts a held flat on the map for resale or sublease; the valley approves; the offer shows on the flat and on the map |
| stage | proposed · built and green on a bench with throwaway keys; the gate is not installed on the front until the owner says so |

## how it works

an act is a short text a wallet signs with `personal_sign` (EIP-191). the gate recovers the signer, checks the right, and commits the act as one file to the acts repository, the only thing it can write. the state of the market is the fold of those files in the order they were committed: who holds a flat, whom a holder gave a mandate, what is offered, what the valley approved. there is no database. anyone holding the repository recomputes the same market, and the build does on every push.

| act | who signs | what it does |
|---|---|---|
| role | the root for an admin; an admin for an agent | names an admin, accredits an agent, removes either |
| profile | anyone, for themselves | the name and the contact buyers see |
| claim | anyone | "this flat is mine": asks the valley to bind the address |
| holder | an admin | binds a flat to its holder's address; a flat that changes hands drops its mandates and offers |
| mandate | the flat's holder | gives one agent the right to offer one flat: resale, sublease or both, a commission, an end date |
| revoke | the holder, the agent, an admin | ends a mandate; the agent's offers on that flat come down |
| listing | the holder, or an agent with a live mandate for that deal | the offer: resale or sublease, price, term, words, contact; a new offer replaces the old one |
| approve · reject | an admin | an offer goes live, or not, with a reason |
| withdraw | the offer's signer, the holder, an admin | takes an offer down |

the root is the owner's address, the one the apparatus pins. it alone names admins.

## the three doors

- `/sell` — the funnel for a holder and for an agent, five steps: wallet, the flat, the offer, who sells, the track. a holder who has an agent signs a mandate; the agent opens the same page with their own wallet and lists the flat.
- `/admin` — the valley's desk: offers and claims that wait come first, then the books of holders, mandates, admins and accredited agents. every button is a signed act.
- the map and the flat's page — a live offer shows as RESALE or SUBLEASE with its price and the way to its seller: the holder, or the agent, by the name and contact they signed. no address leaves the gate.

## parts

| part | where |
|---|---|
| the acts, their rights, the fold | `gate/acts.py` |
| the verifier: keccak-256, secp256k1 recovery, EIP-191 — apparatus's arithmetic, unchanged | `gate/sign.py` |
| the gate: sign-in, acts in, commits out, the public file | `gate/gate.py` |
| tests: 17, incl. the bench's finding (acts of one second must fold in commit order) | `gate/acts_test.py` |
| install on the front: one deploy key, runit, one nginx location | `gate/front.sh` |
| the client: sign-in, one act, the views | `src/market.rs`, `src/eth.rs` (`personal_sign`) |
| the funnel, the desk, the offers | `src/sell.rs`, `src/market_admin.rs`, `src/offers.rs` |

## measured on the bench

a local gate with four throwaway keys (root, admin, holder, agent), the built site beside it, a headless browser with a wallet that signs through the bench:

- an agent without a mandate is refused; with one, lists; a holder is refused when approving; the admin approves; the public file carries the offer with the agent's name and no address
- the holder signs in on `/sell`, sees the flat bound to the address, signs an offer; the admin signs in on `/admin`, approves; the flat's page shows `RESALE · $39,000 · by its holder`, the map lists it under FROM HOLDERS
- `gate/acts.py verify` re-folds the committed files: 9 acts, 0 refused

## open

- a flat bound to a holder still reads its status from the lease book; the two meet when the valley publishes the deal as owned
- a buyer's request goes to the seller's Telegram; it is not an act yet, so the desk does not see leads
- the commission is in the mandate and is not shown to buyers
- the acts repository is private (it carries addresses); the build's verify step runs there
- installing on the front: a new private repository, one deploy key, one service, one nginx location — the owner's word
