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

## Ribbon, nebo Mac skořápka

Výchozí je **ribbon** (`brand.toml` › `[ui] shell = "ribbon"`): upstreamové UI ve stylu Wordu,
stejně jako WPS Office, který s ribbonem v Mac App Store je. Systémový menu bar má appka v obou
variantách. `shell = "mac"` přepne na rozhraní ve stylu macOS popsané níže. Obě varianty
spouštějí stejné příkazy, takže funkce, undo, vyhledávání příkazů, makra, MCP i řídicí kanál
fungují beze změny.

**Neutrální názvy** (`[ui] neutral_terms = true`, tabulka `brand/terms.tsv`): ochranné známky
Microsoftu v popiscích se nahrazují. WordArt → Text Art, SmartArt → Smart Graphic, Researcher →
Research, Immersive Reader → Reading Focus, „Office 2003“ → Classic. Ribbon sám o sobě problém
není, ochranné známky ano: na stížnost vlastníka Apple appku stáhne. `false` vrátí původní
názvy.

| Část | Soubor | Co dělá |
|---|---|---|
| Lišta | `crates/ui-egui/src/mac_shell/toolbar.rs` | Jeden řádek v pásu se semafory: osnova, název dokumentu („Upraveno“), styl odstavce, B/I/U, seznamy, tabulka, obrázek, tvary, Vložit, komentář, inspektor |
| Inspektor | `mac_shell/inspector.rs` | Panel vpravo, záložky Text / Stránka / Dokument |
| Menu bar | `mac_shell/menu.rs` (model) + `apps/wordcraft/src/mac_menu.rs` (AppKit přes `muda`) | Systémová menu: aplikace, Soubor, Úpravy, Vložit, Formát, Zobrazení, Nástroje, Okno, Nápověda; vše ostatní přes Nápověda › Všechny příkazy |
| Start | `mac_shell/start.rs` | Výběr šablony a nedávné dokumenty místo modrého panelu „Good morning“ |

Upstreamový kód volá skořápku jen na třech místech ve `WordApp::ui` (`mac_shell::on()`).
Upstreamové UI testy (přes 800) běží dál nad ribbonem; skořápka má vlastní testy v
`mac_shell/tests.rs`. Položky menu nemají vlastní klávesové zkratky, takže zkratky obsluhuje dál
aplikace sama a nic se nezdvojuje.

Zatím zůstává z upstreamu: stránky Soubor › Možnosti / Tisk / Export / Info (otevírají se z menu),
pravítko a stavová lišta. Výchozí písmo nových dokumentů je dál `Aptos` (konstanta v
`crates/doc/src/styles.rs`, změna by zasáhla layout i testy upstreamu).

## Mac App Store

Stejná binárka se dá vydat dvěma cestami:

| | DMG (mimo App Store) | Mac App Store |
|---|---|---|
| Skript | `packaging/macos/package.sh` | `brand/appstore/package.sh` |
| Podpis | Developer ID + notarizace | Apple Distribution + provisioning profil, `.pkg` podepsaný Mac Installer Distribution |
| Sandbox | ne | ano (`brand/appstore/entitlements.plist`) |
| CI | Actions › fork › Run workflow › `dmg` | Actions › fork › Run workflow › `appstore` |

Co fork kvůli sandboxu doplnil:
- **Ukládání** (`crates/engine/src/io/sandbox_save.rs`): sandbox nedovolí dočasný soubor vedle
  dokumentu, takže se zapisuje na místo. Původní obsah se předtím zkopíruje do kontejneru appky
  a při chybě se vrátí zpět.
- **Nedávné dokumenty** (`apps/wordcraft/src/sandbox_access.rs`): security-scoped bookmarky přes
  auditovaný crate `apple-scoped-bookmarks`, takže po restartu jde dokument z Nedávných otevřít
  i uložit.
- Nastavení, šablony, fonty a slovníky se v sandboxu automaticky ukládají do kontejneru
  (`~/Library/Containers/<bundle_id>/`).
- **Zotero** funguje: entitlement `network.client` povoluje odchozí spojení, appka ho používá
  jen k Zoteru na stejném Macu (`127.0.0.1:23116`). Řídicí port pro agenty (MCP, `--control`)
  potřebuje `network.server`, ten App Store verze nemá, takže zůstává jen v DMG.

### Jednorázová příprava (tvoje kroky)

1. developer.apple.com › Identifiers: zaregistruj App ID `cz.praguepixels.writer` (macOS).
2. Certificates: vytvoř **Apple Distribution** a **Mac Installer Distribution**, nainstaluj je do
   Klíčenky a exportuj obojí do jednoho `.p12`.
3. Profiles: **Mac App Store** profil pro to App ID a certifikát Apple Distribution.
4. App Store Connect › Aplikace › +: nová macOS appka s bundle ID `cz.praguepixels.writer`.
   Název appky v App Store musí být unikátní, takže tady už potřebuješ finální jméno.
5. Users and Access › Integrations: API klíč s rolí App Manager (pro automatický upload).
6. GitHub › Settings › Secrets: `MAS_CERTIFICATES` (base64 `.p12`), `MAS_CERTIFICATES_PASSWORD`,
   `MAS_PROFILE` (base64 profilu), `APPLE_TEAM_ID`, `MAS_APP_IDENTITY`
   (např. `Apple Distribution: Prague Pixels s.r.o. (TEAMID)`), `MAS_INSTALLER_IDENTITY`
   (`3rd Party Mac Developer Installer: Prague Pixels s.r.o. (TEAMID)`) a pro upload
   `ASC_KEY_ID`, `ASC_ISSUER_ID`, `ASC_KEY` (obsah `.p8`). Base64 vytvoříš příkazem
   `base64 -i soubor | pbcopy`.
7. Actions › fork › Run workflow › `appstore`. Bez API klíče stáhneš `.pkg` z artefaktů a
   nahraješ ho přes Transporter.

### Při odeslání do review

- **Šifrování (export compliance):** appka otevírá heslem chráněné `.docx` (AES). V App Store
  Connect na to odpovídáš v dotazníku. Klíč `ITSAppUsesNonExemptEncryption` fork záměrně
  nenastavuje, rozhodnutí je na tobě.
- **Popis a klíčová slova:** nepoužívej „Word“, „Microsoft“ ani „Office“ (pravidlo 4.1). „Opens
  .docx files“ je v pořádku.
- **Poznámka pro review:** appka funguje offline, bez účtu a bez přihlášení. K síti: „The
  network.client entitlement is used only for the optional Zotero integration, which connects
  to the Zotero desktop app on the same Mac (localhost). No data leaves the device.“
- **Privacy (App Privacy v App Store Connect):** Data Not Collected — appka nic neodesílá.

Ještě neověřeno na skutečném Macu v sandboxu: Číst nahlas (spouští systémový `say`), tlačítko
„nastavit jako výchozí appku“ a Zotero s nainstalovaným Zoterem.

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
