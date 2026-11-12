# Third-party notices

## 3D models (bundled in `client/public/models/`)

| Asset | Author | Licence | Files |
|---|---|---|---|
| Watercraft Kit 2.1 | Kenney (www.kenney.nl) | CC0 1.0 (public domain) | `models/boats/*.glb`, `models/boats/Textures/colormap.png`, licence in `models/boats/LICENSE-Kenney-Watercraft.txt` |
| Mini Characters | Kenney (www.kenney.nl) | CC0 1.0 (public domain) | `models/crew/*.glb`, `models/crew/Textures/colormap.png`, licence in `models/crew/LICENSE-Kenney-MiniCharacters.txt` |

The boat pictures in `models/boats/previews/` are the Watercraft Kit's own preview images (same licence). CC0 needs no attribution;
Kenney is credited here anyway.

Everything else visual is made in code by this project:
- the river, banks, plants, plastic, logs, weed mats, pipes and waves;
- the stand-in boats shown while the models load;
- the logo and the UI icon set.

## Audio (bundled in `client/public/audio/`)

| Asset | Author | Licence | Used for |
|---|---|---|---|
| [40 CC0 water / splash / slime SFX](https://opengameart.org/content/40-cc0-water-splash-slime-sfx) | rubberduck (OpenGameArt) | CC0 1.0 | `sfx/pickup-*`, `hop-*`, `capsize-*`, `wave-*`, `cut-*`, `slowed-*` |
| [Sea and river wave sounds](https://opengameart.org/content/sea-and-river-wave-sounds) | RandomMind (OpenGameArt) | CC0 1.0 | `sfx/river-loop.mp3`, a 45-second seamless loop cut from the "Vistula" recording |

| [Interface Sounds](https://kenney.nl/assets/interface-sounds) | Kenney | CC0 1.0 | `sfx/click-1` (click_001), `countdown-1` (toggle_001), `go-1` (confirmation_001), `ping-1` (drop_002), `plugFail-1` (error_006) |
| [Impact Sounds](https://kenney.nl/assets/impact-sounds) | Kenney | CC0 1.0 | `sfx/bump-1` (impactPlank_medium_000) |
| [Music Jingles](https://kenney.nl/assets/music-jingles) | Kenney | CC0 1.0 | `sfx/win-1` (STEEL02), `lose-1` (STEEL01), `plug-1` (STEEL15), `rule-1` (STEEL06), `boost-1` (PIZZI16) |
| [Music Loop Bundle](https://tallbeard.itch.io/music-loop-bundle) | Abstraction (Tallbeard Studios) | CC0 1.0 | `music/hub.mp3` ("Interior Birdecorator Explore"), `music/race.mp3` ("Sketchbook 2024-07-04") |

The files were converted to MP3 and their volume levelled; nothing else was changed. Abstraction asks, but
does not require, that games earning money credit "Abstraction" with a link to the bundle page and consider
supporting the project.

## Font (loaded at runtime, not bundled)

**Fredoka**, SIL Open Font License 1.1, is loaded from Google Fonts by `client/index.html`. Without internet access
the game falls back to the system's rounded or sans-serif font.

## Client packages (bundled into `client/dist`)

| Package | Version | Licence |
|---|---|---|
| @babylonjs/core | 9.32.1 | Apache-2.0 — https://github.com/BabylonJS/Babylon.js |
| @babylonjs/loaders | 9.32.1 | Apache-2.0 — glTF loader for the models |

Development only: vite (MIT), vitest (MIT), typescript (Apache-2.0).

## Rust crates (compiled into the server)

Exact versions are pinned in `Cargo.lock`. The main dependencies and their licences:

| Licence | Crates |
|---|---|
| MIT OR Apache-2.0 | actix-web, actix-ws, actix-files, actix-cors, sqlx, serde, serde_json, uuid, chrono, argon2, rand, anyhow, thiserror |
| MIT | tokio, jsonwebtoken, tracing, tracing-subscriber, dotenvy |
| Apache-2.0 OR ISC OR MIT | rustls, rustls-pemfile |

For the full list, run:

```bash
cargo tree --prefix none --format "{p} {l}"
```

## Earlier board game content

`content/journal.json` belongs to the earlier board game, which the client no longer uses. Its text cites the
UNESCO World Heritage Centre, the IUCN Red List and the Nature Conservation Foundation. The text is original; no
source text is reproduced.
