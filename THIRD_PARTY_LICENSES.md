# Tredjepartsindhold i Huginn and Muninn

Programmets egen kode er under Apache-2.0-licensen (se `LICENSE` og `NOTICE`). Det følgende er ikke skrevet til programmet.

## Wi24rd UI Kit 2 (`ui/assets/kit2/`)

Kopieret uændret fra pakken Wi24rd UI Kit 2 (versionen står i `ui/assets/kit2/VERSION`; filernes SHA-256 i `PAKKE.json`, tjekket af `npm run kit:tjek`).

| Del | Filer | Licens | Ophav |
|---|---|---|---|
| Tokens og komponenter | `tokens.css`, `kit.css`, `kit.js`, `tema.js`, `sprogmotor.js` | Apache License 2.0 — fuld tekst i `ui/assets/kit2/LICENSE` | Wi24rd (samme ophav som dette program; udskilt fra Wi24rd Grimnir) |
| Inter (variabel) | `fonts/inter-latin-wght-normal.woff2`, `fonts/inter-latin-ext-wght-normal.woff2` | SIL Open Font License 1.1 — fuld tekst i `ui/assets/kit2/fonts/OFL-Inter.txt` | Copyright 2016 The Inter Project Authors (https://github.com/rsms/inter) |
| JetBrains Mono (variabel) | `fonts/jetbrains-mono-latin-wght-normal.woff2`, `fonts/jetbrains-mono-latin-ext-wght-normal.woff2` | SIL Open Font License 1.1 — fuld tekst i `ui/assets/kit2/fonts/OFL-JetBrainsMono.txt` | Copyright 2020 The JetBrains Mono Project Authors (https://github.com/JetBrains/JetBrainsMono) |

## Kortdata (`ui/assets/verden.json`, `ui/assets/colo.json`)

Kildeangivelsen står også i programmet (Værnet, under "Sådan tælles der").

| Data | Kilde | Licens |
|---|---|---|
| Landkonturer (`verden.json` → `land`, `flader`, `flader_uden_kode`) | Natural Earth 110m via world-atlas | Public domain |
| Landenes midtpunkter (`verden.json` → `lande`) | world-countries | ODbL |
| Datacentrenes koordinater (`verden.json` → `colo`, `net`) | OpenFlights via airport-codes (ifølge filens egen note) | Ikke afgjort |
| Tidszone → land (`tidszoner.json`) | tz-databasens `zone.tab`, genereret med `tools/tidszoner.mjs` | Public domain |
| Datacentre → by (`colo.json`) | wi24rd.com (samme ophavsmand) | MIT efter ophavsmandens beslutning 2026-11-03 |

## Kode fra søsterprojekter

`ui/assets/vagt.js`, `ui/assets/*.css` og `src-tauri/src/vaern.rs` er porteret fra wi24rd.com-motoren (samme ophavsmand, MIT).

## Rust-crates til menulinjen, én instans og start ved login

Crates, der kom med i `src-tauri/Cargo.lock` med menulinjen. Licensen er slået op i hver crates egen `Cargo.toml` (feltet `license`) i den låste version. Menulinjeikonet bruger `tray-icon`, der allerede var med via `tauri` (featuren `tray-icon`); den gav ingen nye crates.

| Crate | Version | Licens | Hvorfor | Platform |
|---|---|---|---|---|
| `tauri-plugin-single-instance` | 2.5.5 | Apache-2.0 OR MIT | Kun én kørende instans | Alle desktop |
| `tauri-plugin-autostart` | 2.7.0 | Apache-2.0 OR MIT | Start ved login | Alle desktop |
| ↳ `auto-launch` | 0.6.0 | MIT | LaunchAgent-plist / Run-nøgle | Alle desktop |
| ↳ `dirs` | 6.0.0 | MIT OR Apache-2.0 | Hjemmemappen (`~/Library/LaunchAgents`) | macOS, Linux |
| ↳ `os_info` | 3.15.0 | MIT | macOS-version (auto-launch) | Alle desktop |
| ↳ `nix` | 0.31.3 | MIT | Via `os_info` | macOS, Linux |
| ↳ `smappservice-rs` | 0.1.3 | MIT | SMAppService (linkes med, bruges ikke i LaunchAgent-tilstand) | macOS |
| ↳ `objc2-service-management` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | Via `smappservice-rs` | macOS |
| ↳ `objc2-security` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | Via `smappservice-rs` | macOS |
| ↳ `windows-registry` | 0.6.1 | MIT OR Apache-2.0 | Run-nøglen | Windows |
| ↳ `windows-result` | 0.4.1 | MIT OR Apache-2.0 | Via `windows-registry` (var allerede i træet) | Windows |
| ↳ `tracing-attributes` | 0.1.31 | MIT | Makro via `tracing` (single-instance) | Alle desktop |
| ↳ `tauri-plugin` | 2.7.2 | Apache-2.0 OR MIT | Kun ved bygning (pluginnernes `build.rs`) | — |

Kun på Linux (via `zbus` i single-instance), ikke i macOS- eller Windows-buildet (`cargo tree --target …`): `zbus`, `zbus_macros`, `zbus_names`, `zvariant`, `zvariant_derive`, `zvariant_utils`, `zcheapstr`, `endi` (MIT); `async-broadcast`, `async-channel`, `async-executor`, `async-io`, `async-lock`, `async-process`, `async-recursion`, `async-signal`, `async-task`, `async-trait`, `blocking`, `concurrent-queue`, `enumflags2`, `enumflags2_derive`, `errno`, `event-listener`, `event-listener-strategy`, `futures-lite`, `ordered-stream`, `parking`, `piper`, `polling`, `signal-hook-registry` (MIT OR Apache-2.0); `rustix`, `linux-raw-sys` (Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT).

Står i `Cargo.lock`, men bygges hverken på macOS, Windows eller Linux (x86_64): `hermit-abi`, `tempfile` (MIT OR Apache-2.0), `uds_windows` (MIT).

## Rust-crates til lokale notifikationer

Crates, der kom med i `src-tauri/Cargo.lock` med notifikationerne (diff mod forrige `Cargo.lock`: præcis disse fire). Licensen er slået op i hver crates egen `Cargo.toml` i den låste version (`tauri-winrt-notification`: crates.io, da den kun hentes til Windows). Pluginnets øvrige afhængigheder (`rand`, `time`, `serde_repr`, `url`, `log`, `thiserror`, på Linux `zbus`) var allerede i træet. Notifikationerne vises af styresystemet; ingen af dem laver netværkskald.

| Crate | Version | Licens | Hvorfor | Platform |
|---|---|---|---|---|
| `tauri-plugin-notification` | 2.8.0 | Apache-2.0 OR MIT | Lokale notifikationer og klik på dem | Alle desktop |
| ↳ `notify-rust` | 4.18.2 | MIT OR Apache-2.0 | Notifikationen pr. platform | Alle desktop |
| ↳ `mac-notification-sys` | 0.6.15 | MIT/Apache-2.0 | macOS' notifikationscenter (Objective-C, bygget med `cc`) | macOS |
| ↳ `tauri-winrt-notification` | 0.8.1 | MIT OR Apache-2.0 | Windows-toasts | Windows |
