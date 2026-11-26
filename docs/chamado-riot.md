# Chamado no suporte de desenvolvedores da Riot (rascunho)

**Subject:** "Selected app type is not available" when registering a new product (Personal and Production)

**Body:**

Hi,

I'm trying to register a new product in the Developer Portal, but both product types fail. After accepting the policies and filling in the New Product Application form (Product Game Focus: League of Legends, not organizing tournaments), Submit returns to the Product Type page with "Failed to create application! Selected app type is not available". It happened with Production API Key and with Personal API Key, and the product was not created. My account currently has one other product with a development key.

The product is Lobby Companion (https://nickmaglowsch.github.io/lobby-companion/, source: https://github.com/nickmaglowsch/lobby-companion), a free, open-source Windows companion for champion select. No ads, no paid tier. It does not use the Riot Games web API and ships no API key. It uses:

- League Client API (LCU): reads game flow, champion select (only champion ids, pick intents, assigned positions and bans of other players; never names, Riot IDs or PUUIDs), perks, own mastery and game data; writes only our own "LC:" rune page and item set, summoner spells via my-selection, and a click-to-ban (only on the player's explicit click, only for the local player's own in-progress ban action, never for picks).
- Planned: a small optional in-game overlay using only the Game Client API (Live Client Data), showing only the player's own data: gold until the next item of the imported build, CS per minute, and inhibitor respawn timers from the event feed. No memory reading.

Questions:
1. How can I register this product, given that both app types fail?
2. Which product type fits a public app that only uses the League Client API and the Game Client API, with no Riot API key?
3. Are the planned overlay features and the click-to-ban acceptable under the policies?

Thanks!
