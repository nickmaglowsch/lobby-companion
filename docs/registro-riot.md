# Registro no Riot Developer Portal (rascunho)

Texto em inglês para o formulário de "Register Product" (produto público / Production).
Os campos exatos do formulário só aparecem depois do login; adapte cada bloco ao campo certo.

---

**Product name:** Lobby Companion

**Product type:** Production (public product, larger scale)

**Game:** League of Legends

**Website:** https://nickmaglowsch.github.io/lobby-companion/

**Source code:** https://github.com/nickmaglowsch/lobby-companion (open source)

**Platform:** Windows desktop app (Tauri 2 + Rust + SvelteKit). Free, no ads, no paid tier, no account.

## Short description

Lobby Companion is a lightweight desktop companion for champion select. When the player locks in a champion, it imports runes, summoner spells and an item set built from aggregated public statistics, and shows ban and pick suggestions for the player's own role. We are adding a small in-game overlay that only uses the official Game Client API (Live Client Data) and only shows the player's own information.

## Features (current, champion select only)

- Imports a rune page (named `LC: <champion>`), summoner spells and an item set (`LC:` prefix) for the player's own locked champion. Only our own pages/sets are replaced; the player's pages are never touched unless they opt in.
- Shows win rate per item, core items, first back, situational items, skill order, rune alternatives, 30-day trend and damage profile, all from public aggregated statistics (lolalytics, cached 6 h, at most 1 request per second, with a link back to the source page).
- Ban suggestions based on the champions the player's own team has picked or declared; pick suggestions for the player's role using counter, synergy and the player's own champion pool (own mastery).
- Team damage composition (physical/magic) of both teams, using only the champions visible in champion select.
- Player-made rune pages stored locally.
- Click-to-ban: during the player's own ban turn, clicking a suggested ban selects and locks that ban in the client. It only happens on an explicit click by the player, only for the local player's own in-progress ban action, and it refuses champions that are already banned, picked or declared. It is never used for picks, never runs on its own, and nothing else is automated (no auto-accept, no auto-pick, no role swaps, no dodges).

## Planned feature: in-game overlay

A small, optional overlay for the player's own game, using only the official Game Client API (`https://127.0.0.1:2999/liveclientdata/`). No memory reading, no injection, no packet inspection.

- **Next item / power spike:** shows how much gold the player still needs for the next item of the imported build (own `currentGold` and own items), as a reminder to plan the next back. It only highlights; the player decides when to recall and what to buy.
- **CS per minute:** the player's own creep score divided by game time.
- **Inhibitor respawn timers:** from the inhibitor-destroyed events in the Live Client Data event feed, which are announced in game to all players.

The overlay never shows enemy cooldowns, enemy summoner spells, jungle camp timers, information about enemies that is not on screen, or anything that is not already available to the player. No ads in the overlay.

## Riot APIs used

- **Riot Games API (web):** none today. No API key is shipped in the app.
- **League Client API (LCU, local only):**
  - Read: `/lol-gameflow/v1/gameflow-phase`, `/lol-gameflow/v1/session`, `/lol-champ-select/v1/session` (only champion ids, intents, assigned positions and bans of other players; the local player's own champion, position and spells), `/lol-perks/v1/pages`, `/lol-perks/v1/currentpage`, `/lol-perks/v1/inventory`, `/lol-summoner/v1/current-summoner`, `/lol-champion-mastery/v1/local-player/champion-mastery`, `/lol-game-data/assets/...`, `/riotclient/region-locale`, and the WebSocket events for game flow and champion select.
  - Write (allow-list enforced in code): `POST /lol-perks/v1/pages`, `DELETE /lol-perks/v1/pages/{id}` (only our `LC:` page), `PUT /lol-perks/v1/currentpage`, `PUT /lol-item-sets/v1/item-sets/{summonerId}/sets` (only replaces our `LC:` sets), `PATCH /lol-champ-select/v1/session/my-selection` (summoner spells), `PATCH /lol-champ-select/v1/session/actions/{id}` (click-to-ban only, local player's own ban action).
- **Game Client API (planned overlay):** `/liveclientdata/activeplayer`, `/liveclientdata/playerlist` (only the local player's items and score), `/liveclientdata/eventdata`, `/liveclientdata/gamestats`.

## Player privacy

The app never reads, shows or stores other players' names, Riot IDs or PUUIDs. The client auth token is never logged or stored. All data stays on the player's PC.

## Contact

Through the Developer Portal messages.
