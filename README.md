# RossBot

A discord bot for asynchronous custom Gartic Phone modes gameplay.

Currently supported modes:
- Rossmode - character creation from attributes

# Infrastructure

Discord bot in Rust using poise.
Surrealdb as game logic storage.
Discord as UI and image store.

# Unordered roadmap

- [x] MVP - Playable Ross mode
- [x] Upgrade database schema - don't match players with matches they previously played
- [ ] Complete command suite - reroll
- [x] Moderation layer - permission and filtration of invalid submissions
- [ ] Add NSFW variants
- [ ] Add another mode - Evolution
- [ ] Duplicate image prevention
- [ ] Game tagging system
- [x] Auxilary commands - get incomplete games, fetch random attributes, etc.
