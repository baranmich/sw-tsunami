# Fork: macOS verze WordCraftu pod vlastní značkou

Tento repozitář je fork [storytold/wordcraft](https://github.com/storytold/wordcraft)
(MIT OR Apache-2.0) pod značkou Prague Pixels. UI a funkce jsou upstreamové; fork mění identitu
aplikace a balení pro macOS a průběžně přebírá upstream.

## Přejmenování aplikace (jediné místo)

Všechno, co uživatel vidí, se bere z [`brand.toml`](brand.toml):

| Co | Odkud |
|---|---|
| název okna, menu bar, Dock, Finder, About, DMG | `[brand] family` + `product` (nebo `name`) |
| texty rozhraní ve všech 27 jazycích | stejně: překladová funkce nahrazuje upstreamový název za běhu |
| metadata souborů (.docx Application, PDF Creator/Producer, ODT, RTF, HTML, LaTeX) | stejně |
| odkazy v About a tlačítko komunity | `website`, `community_url`, `source_url` (prázdné = skryté) |
| ikona | `brand/icon/icon.svg` → `brand/icons.sh` |

Postup:

1. Uprav `brand.toml` (pracovní název je `PP Writer`).
2. Ikona: nahraď `brand/icon/icon.svg` a spusť `brand/icons.sh` (potřebuje `resvg` a Pillow).
3. `cargo xtask brand`, tím ověříš, že nikde nezůstal starý název.
4. Build: `packaging/macos/package.sh --arch universal` (na Macu) nebo GitHub Actions › fork ›
   Run workflow.

`[identity] bundle_id` a `data_dir` **neměň po prvním vydání**: macOS na ně váže nastavení,
oprávnění ke složkám a „Otevřít v“. Proto neobsahují zobrazovaný název a přejmenování je bezpečné.

## Jak je to postavené (kvůli čistým mergům)

- `crates/brand`: konstanty generované z `brand.toml` (`NAME`, `BUNDLE_ID`, `DATA_DIR`, …),
  makro `brand_name!()` pro `concat!` a funkce `rebrand()` pro překladový hook.
- Upstreamový kód je upravený jen na ~60 místech, kde název jde k uživateli mimo `tl!()`.
  Názvy crate (`wordcraft-*`), binárky v `target/` a interní proměnné (`WORDCRAFT_*`) zůstávají,
  uživatel je nevidí a přejmenování by rozbilo každý merge.
- `packaging/macos/package.sh` a `Info.plist.in` čtou `brand.toml` přes `brand/brand.sh`.
  Rozložení okna DMG generuje `brand/dmg.py` nad nezměněným upstreamovým `generate.py`.
- `cargo xtask brand` hlídá, že žádný nový upstreamový řetězec neukazuje starý název. Výjimky
  jsou záměrně v `brand/audit-allow.txt` (poděkování upstreamu v About, kód jen pro Windows).

## Převzetí novinek z upstreamu

```sh
brand/sync-upstream.sh            # nejnovější upstream/main
brand/sync-upstream.sh v0.11.0    # konkrétní verze
```

Skript sloučí upstream, sám vyřeší konflikty v upstreamových workflow (fork je smazal) a na
konci spustí `cargo xtask brand`. Když vypíše nová místa, obal je `wordcraft_brand::NAME` /
`brand_name!()` stejně jako ostatní.

## Licence

Upstream je MIT OR Apache-2.0. Fork zachovává `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` a
`ATTRIBUTION.md` a v About uvádí původ. Značka ArtCraft (`docs/brand/`) do aplikace nejde.
