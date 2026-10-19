# Gaze Grab

Prototyp ovládání kurzoru pravou rukou, gest jedné i obou rukou a diagnostiky očí, bez headsetu. Rust zpracovává signály a stavový automat, nativní AppKit/AVFoundation zajišťuje kameru a malé plovoucí okno. Apple Vision detekuje klouby ruky a obličejové/oční body na běžném Macu (macOS 12+).

## Spuštění

```sh
./scripts/install-gaze-grab.sh
```

Aplikace se nainstaluje do `/Applications/Gaze Grab.app`. V **Rustletu → Gaze Grab → Nastavení…** otevřeš diagnostiku; samostatně je dostupná také přes ikonu oka v horní liště. Nástroj se přidává do `manager/resources/catalog.json`.

1. Otevři diagnostiku a klikni na **Zapnout kameru**. macOS při prvním použití požádá o přístup ke kameře. V rozbalovacím seznamu lze vybrat jinou kameru.
2. Ukaž jednu nebo obě celé otevřené ruce. Pravá je tyrkysová, levá fialová; popisky jsou anatomické a nezávislé na straně obrazu. Každá má vlastní stav gest. Ukaž celou otevřenou ruku. Po 100 ms platného otevření je systém připravený.
3. Spoj **palec, ukazovák a prostředník**. Stabilní sevření po 150 ms potvrdí **UCHOPENÍ**, špičky zezelenají a při pohybu se kreslí čára od počátečního místa uchopení.
4. Prsty od sebe oddal a nech je otevřené 100 ms. Zobrazí se **PUŠTĚNÍ**.
5. Tlačítko **Vypnout kameru**, zavření diagnostického okna, uspání nebo odhlášení relace vypnou snímání. Spuštění nástroje správcem kameru samo nezapíná. Kamera se po probuzení sama znovu nezapne.

## Kurzor, klik a scroll

Ve výchozím zkušebním režimu se neposílají události myši. Bílý kříž v náhledu ukazuje polohu virtuálního kurzoru.

- **Pohyb:** relativní pohyb pravé dlaně posouvá kurzor z aktuálního místa na hlavním monitoru jako trackpad. Ruku nech v přirozené poloze a hýbej jen v malém prostoru. Pomalý pohyb je přesný, rychlý má větší dosah díky akceleraci. Citlivost upravuje **Rychlost kurzoru**. Klidová poloha kurzor neposouvá a zachycení ruky po výpadku jej nepřemístí. Po získání pravé ruky se čeká 300 ms na stabilní sledování.
- **Klik:** natáhni jen ukazovák, ostatní tři prsty ohni. Po 220 ms se klik připraví. Krátce zatlač ukazovák ke kameře: dostatečné zkrácení jeho projekce vůči šířce dlaně po alespoň 60 ms vyvolá jeden klik. Další vyžaduje znovu natažený ukazovák a uplynutí prodlevy 700 ms. Posuvník **Citlivost kliku** mění požadované zkrácení (vyšší hodnota = menší nutná změna).
- **Alternativní klik:** v nabídce **Klik** nebo v průvodci vyber **Spojení palce a ukazováku**. Oddal špičky, počkej na Připraveno, spoj je a zase oddal. Tato volba nevyžaduje odhad pohybu do hloubky.
- **Scroll jednou rukou:** zvedni pravý ukazovák a prostředník, prsteník a malík ohni. Po potvrzení pózy 180 ms pohybuj prsty nahoru/dolů. Kurzor během scrollování stojí.
- **Scroll oběma rukama:** vyber druhou volbu v seznamu **Scroll**. Levou spoj palec, ukazovák a prostředník a drž potvrzené sevření. Pravou s nataženým ukazovákem pohybuj nahoru/dolů. Po puštění levé se pravá vrací k pohybu kurzoru.
- **Systémová myš:** zapni kameru a přepínač **Ovládat systémový kurzor**. Povol aplikaci v macOS → Soukromí a zabezpečení → **Zpřístupnění** a přepínač zapni znovu. Ovládání začíná po 2 sekundách. **Esc**, vypnutí kamery, zavření okna, uspání nebo chybějící výsledky jej vypnou. Přepínač se po spuštění aplikace nikdy sám nezapne.

**Klik je experimentální odhad z 2D kamery, nikoli měření hloubky.** Kamera může zaměnit pohyb ke kameře za ohnutí prstu nebo natočení ruky. Začni zkušebním režimem a uprav citlivost podle své polohy a světla. Systémový drag-and-drop tříprstým sevřením ještě zapojený není.

Nejasná strana ruky nebo dvě detekce označené stejnou stranou danou stranu vyřadí. Výpadek potřebných kloubů, skok polohy nebo časová mezera ruší připravený klik; ruka se musí znovu potvrdit. Při scrollování se klik nevyhodnocuje. Obě ruce lze sledovat současně a jejich tříprstá gesta jsou nezávislá. Volba scrollu, způsob kliku, rychlost kurzoru, citlivost kliku a práh sevření se ukládají lokálně.

## Průvodce gesty

Tlačítko **Naučit se gesta…** v diagnostice otevře pět kroků s živým náhledem: rozpoznání pravé ruky, malý relativní pohyb, tři kliky, scrollování a společné použití obou rukou. Průvodce používá stejný detektor jako běžné ovládání, počítá skutečné události a ukazuje přípravu i průběh kliku. Tlačítko Další gesto se zpřístupní po úspěchu; každý krok lze také přeskočit nebo zopakovat tlačítkem Zpět. V kroku kliku lze upravit citlivost nebo přepnout na spojení dvou prstů.

Otevření průvodce vypíná systémové ovládání. Cvičný kurzor a kliky zůstávají v náhledu, takže pokusy neklikají do jiných aplikací. Po zavření průvodce se ovládání samo nezapne. Kamera má stále vlastní vypínač. Průvodce se nabídne automaticky při prvním běžném spuštění a lze jej kdykoli znovu otevřít tlačítkem.

## Vlastní naučená gesta

V diagnostice nebo v menu otevři **Vlastní gesta…**. Kamera i učení běží lokálně.

1. Zapni kameru, zadej název, vyber **pravou / levou / obě ruce** a typ: **Póza (podržet)** nebo **Pohyb (zopakovat)**. Vyber akci a stiskni **Připravit gesto**.
2. Stiskni **Nahrát ukázku 1/3**. Po dvousekundovém odpočtu drž pózu nebo za dvě sekundy proveď celé gesto. Všechny potřebné prsty musí zůstat viditelné. Opakuj pro další dvě ukázky; pohyb začínej ze stejné výchozí polohy.
3. Klikni na **Uložit gesto**. V náhledu zopakuj naučené gesto. Uvidíš jeho jméno při rozpoznání. Před dalším vyvoláním se vrať do neutrální polohy, aby se gesto odjistilo.
4. Uloženému gestu lze přiřadit **klik, dvojklik, pravé tlačítko, scroll nahoru/dolů, pozastavení/obnovení kurzoru** nebo **jen rozpoznávání**. Při změně akce nebo vypnutí rozpoznávání stiskni **Uložit přiřazení**. Gesta lze smazat a nově naučit.
5. Zavři okno učení a zapni systémové ovládání, aby se přiřazené akce vykonávaly. Během učení a zkoušení v jeho okně se akce do jiných aplikací neposílají. Naučené gesto s přiřazenou akcí má při rozpoznání přednost před vestavěným klikem a scrollem. Gesto pro pozastavení lze znovu použít k obnovení; Esc celé ovládání vypne.

Jde o učení z několika ukázek, ne o univerzální rozpoznávání libovolných gest. Normalizované body rukou tvoří osobní šablony: póza porovnává tvar prstů, pohyb také dráhu zápěstí pomocí časového zarovnání (DTW). U obou rukou se hodnotí i vzájemná poloha. Poloha v obrazu a vzdálenost od kamery se normalizují; podobný úhel ruky a dobré světlo pomáhají. Výrazně rozdílné ukázky nebo téměř totožné uložené gesto se odmítnou. Nejednoznačné rozpoznání nespustí akci. Nejvýše lze uložit 32 gest. Chybějící body a časové výpadky nepovažujeme za platnou ukázku.

Šablony a přiřazení se atomicky ukládají do `~/Library/Application Support/Gaze Grab/gestures.json`. Ukládají se souřadnice příznaků, nikoli snímky kamery. Po restartu se načtou. Rozpracované ukázky jsou pouze v paměti a zavřením učení nebo zastavením kamery se zruší. Poškozený či nečitelný soubor se automaticky nepřepisuje.

## Plynulost kurzoru

Posuvník **Vyhlazení kurzoru** odděluje klid pohybu od jeho rychlosti. Vpravo potlačuje chvění výrazněji, vlevo má menší zpoždění. Adaptivní filtr dlaně vyhlazuje pomalý pohyb a při rychlém zvyšuje odezvu. Systémový kurzor se navíc posouvá v 60Hz krocích mezi výsledky kamery, takže nesleduje jen jednotlivé příchody snímků. **Rychlost kurzoru** dál nastavuje dosah relativního pohybu. Výpadek ruky, scroll, klik nebo vypnutí ovládání ruší neprovedený pohyb, aby kurzor nedojížděl po ztrátě sledování.

## Diagnostika

- Živý zrcadlený náhled a obrysy ruky, očí a zornic ve správných souřadnicích i při změně poměru stran kamery.
- Rozpoznání tříprstého sevření s časovým potvrzením a hysterezí. Samotné spojení palce a ukazováku nestačí.
- Vzdálenosti špiček normalizované šířkou dlaně, jistota potřebných kloubů, počty rukou/obličejů, FPS a délka analýzy.
- Nastavitelný práh sevření (výchozí 0,35 šířky dlaně), uvolnění nad 1,6násobkem prahu.
- Historie posledních čtyř událostí v paměti. Chybějící ruka, potřebné klouby nebo skok polohy zruší uchopení; výpadek sledování není vydáván za záměrné puštění. Při spuštění se sevřenou rukou se čeká na otevření.
- Oční signál z polohy obou zornic vůči obrysům očí. Výpočet kompenzuje natočení obrysů, odmítá zavřené oči, chybějící body a velký nesoulad mezi očima. Při více obličejích se oční signál nepoužije.

**Oční signál je nekalibrovaný příznak z 2D obrazu. Není to ještě poloha pohledu na monitoru.** Obličejové landmarky z běžné kamery nemusí dostatečně přesně odpovídat skutečnému pohledu; mrkání, brýle, světlo, vzdálenost i pohyb hlavy ho ovlivňují. Ukazatel X/Y v diagnostice zobrazuje právě tento signál. Oční signál zatím neřídí systémový kurzor; ten lze ovládat pravou rukou podle postupu výše.

## Další kroky k demu na ploše

1. **Hotovo:** kamera, detekce a diagnostika gest/očí.
2. Kalibrační body na monitoru, odhad a měření chyby pohledu při přirozené poloze hlavy, vyhlazený virtuální kurzor. Ověřit, zda nativní oční příznaky stačí; případně doplnit specializovaný model pro odhad pohledu.
3. Spojení pohledu a ruky: při sevření zamknout cíl pod virtuálním kurzorem, polohu potom posouvat relativním pohybem ruky a pustit po potvrzeném otevření. Nejprve na testovacím plátně.
4. Zapojit systémový drag-and-drop ve Finderu po povolení Zpřístupnění, s jasným zrušením při ztrátě sledování. Ověřit skutečný přesun souboru na ploše.

## Soukromí a provoz

Snímání používá pouze video. Obraz a souřadnice se zpracovávají lokálně v paměti; neukládají se fotografie, video ani diagnostické logy do souborů; vlastní naučené šablony bodů rukou se ukládají podle popisu výše a nic se neodesílá na server. Nástroj nevyžaduje přístup k mikrofonu ani zachycení obrazovky. Zpřístupnění je potřeba jen pro zapnuté ovládání systémové myši, kamera a diagnostika fungují bez něj. Oprávnění kamery vyřizuje standardní dialog macOS. Podepsaný balíček má jen odpovídající camera entitlement.

AVCaptureSession se konfiguruje na vlastní frontě, Vision běží sériově na druhé frontě s limitem 15 analýz za sekundu a zahazováním opožděných snímků. Výsledky po zastavení nebo změně session se ignorují pomocí generace výstupu. UI zobrazuje skutečnou rychlost zpracování; při nedostatku snímků zruší platnost překryvu.

## Ověření

```sh
cargo test -p gaze-grab
cargo run -p gaze-grab -- --vision-probe
cargo clippy --workspace --all-targets -- -D warnings
```

Testy ověřují učení a následné rozpoznání póz i pohybů, obě ruce, odlišný směr pohybu, potlačení opakovaných a nejednoznačných akcí, ukládání/přiřazení/mazání, neúspěšný zápis bez změny modelů, přerušenou ukázku a chybějící klouby, redukci chvění filtrem, relativní pohyb bez počátečního skoku a jeho akceleraci, postupné zatlačení přes mezilehlé polohy, klik vůči délce konkrétního ukazováku, alternativní pinch a opětovné odjištění, nezávislost obou rukou při změně pořadí detekcí, odmítnutí nejasné strany, přípravu/potvrzení/opakování kliku, scroll bez kliku a oba způsoby scrollování, otevření před uchopením, rozlišení dvou/tří prstů, potvrzení a hysterezi, rušení při ztrátě ruky/skoku/změně času, nezávislost vzdáleností na měřítku a poměru stran a odmítnutí mrknutí/chybějících zornic. `--vision-probe` spustí skutečné Vision requesty nad prázdným lokálním snímkem bez použití kamery a ověří, že nevymýšlejí ruku ani obličej.

## Použitá nativní API

[Apple Vision: rozlišení strany ruky](https://developer.apple.com/documentation/vision/vnhumanhandposeobservation/chirality), [ověření Zpřístupnění](https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions), [události scrollování](https://developer.apple.com/documentation/coregraphics/cgeventcreatescrollwheelevent).
