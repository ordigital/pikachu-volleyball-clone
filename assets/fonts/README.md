# Fonty gry

- **Noto Sans Regular** — `NotoSans-Regular.ttf`: pasek menu i okna Controls/About.
  Font dołączony z lokalnego pakietu noto-fonts, licencja `NotoSans-OFL.txt`.

- **Jersey 10 Regular** — `Jersey10-Regular.ttf`: wybór liczby graczy,
  komunikaty i punktacja (obrys zmniejszony o połowę). [Źródło](https://fonts.google.com/specimen/Jersey+10).
- **Balsamiq Sans Bold 700** — `BalsamiqSans-Bold.ttf`: BEACH VOLLEYBALL i wektorowy BATTLE!.
  [Źródło](https://fonts.google.com/specimen/Balsamiq+Sans).

Fonty są wkompilowane w grę; Jersey i Balsamiq dostarczył użytkownik z Google Fonts.
Zachowaj dołączone pliki `*-OFL.txt` przy dystrybucji. Duże logo PIKACHU korzysta z renderu PNG źródła SVG.
Kolory i grubość obrysu są ustawiane w `src/gfx.rs`. `src/text_label.rs` tworzy
podczas uruchamiania maski całych napisów z marginesem; shader `outline.frag`
rysuje równy obrys w jednym przebiegu, również przy pierwszej cyfrze „2”.
Po zmianie fontów wykonaj build.
