# Third-party notices

## Art and audio
All 3D models (terrain, trees, figs, Lantana, grass, rocks, canopy bridges and the
five animals), textures (water normals, nameplate) and UI icons are generated in
code by this project. No third-party art or audio assets are included.

## Client packages (bundled into client/dist)
| Package | Version | Licence |
|---|---|---|
| @babylonjs/core | 9.32.1 | Apache-2.0 — https://github.com/BabylonJS/Babylon.js |

Development only: vite (MIT), vitest (MIT), typescript (Apache-2.0).

## Rust crates (compiled into the server)
Exact versions are pinned in `Cargo.lock`. Main dependencies and their licences:
actix-web, actix-ws, actix-files, actix-cors (MIT OR Apache-2.0); tokio (MIT);
sqlx (MIT OR Apache-2.0); serde, serde_json (MIT OR Apache-2.0); uuid, chrono
(MIT OR Apache-2.0); jsonwebtoken (MIT); argon2 (MIT OR Apache-2.0); rand
(MIT OR Apache-2.0); rustls, rustls-pemfile (Apache-2.0 OR ISC OR MIT); tracing,
tracing-subscriber (MIT); dotenvy (MIT); anyhow, thiserror (MIT OR Apache-2.0).
Run `cargo tree --prefix none --format "{p} {l}"` for the full list.

## Ecology sources
Field Journal text cites the UNESCO World Heritage Centre, the IUCN Red List and
the Nature Conservation Foundation (see `content/journal.json`). The text is
original; no source text is reproduced.
