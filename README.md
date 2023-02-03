# ZPR-GAME-ENGINE

# Wymagane zewnętrzne zależności

- cargo (rust toolchain)
- cmake
- ninja

# Uruchomienie projektu

Będąc w główny katalogu projektu (zpr-game-engine) należy w termianu uruchomić polecenie 'cargo run ścieżka_do_pliku_z_poziomem'. Wczytany zostanie podany plik z poziomem - można skorzystać z jednego z dostarczonych plików: `default.ron`, `default2.ron` lub `default3.ron`.

# Generacja Dokumentacji do Kodu

Będąc w główny katalogu projektu (zpr-game-engine) należy w termianu uruchomić polecenie 'cargo doc' (można wywołać z flagą '--open' w celu natychmiastowego otworzenia dokumentacji).

# Uruchomienie testów

Aby uruchomić testy jednostkowe należy w terminalu wykonać z głównego katalogu projektu polecenie `cargo test`

# Formatowanie oraz linter

Polecenie `cargo fmt` formatuje kod w całym projekcie, natomiast polecenie `cargo clippy` uruchamia linter zgłaszający wszelkie ostrzenieżnia.

# Instrukcja do gry

Celem gry jest zebranie obecnym na ekranie okręgiem ananasa. Przeciągając po ekranie można tworzyć wielokąty, natomiast przytrzymując chwilę w miejscu można rysować dodatkowe okręgi (tylko ten pierwszy zbiera ananasa). Stworzone kształty oraz pierwszy okrąg można łączyć zawiasami bądź wiązaniami. Aby to zrobić należy trzymając odpowiednio D lub S nacisnąć na kształt, a następnie narysować nowy kształt tak, aby nachodził on na uprzednio dodane wiązanie bądź nawias. Trzymając przycisk A można korzystać z gumki do usuwania narysowanych kształtów.
