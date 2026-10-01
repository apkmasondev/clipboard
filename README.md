# Super Clipboard

Lokalny menedżer schowka i biblioteka snippetów dla Windows 11 x64. Tauri 2, React, TypeScript, Rust i SQLite. Bez konta, backendu, telemetrii i usług AI.

Bezpłatny także w pracy zawodowej. Sprzedaż aplikacji i jej przeróbek jest zabroniona zgodnie z [LICENSE](LICENSE). Kod jest dostępny publicznie na licencji source-available; nie jest to licencja open source zgodna z OSI. Licencje zależności pozostają bez zmian.

## Instalacja

Pobierz instalator `Super Clipboard_1.1.0_x64-setup.exe` z [wydań projektu](https://github.com/apkmasondev/clipboard/releases). Instalator działa dla bieżącego użytkownika, zawiera polski interfejs, deinstalator oraz opcjonalny skrót na pulpicie. Aplikacja wymaga Microsoft Edge WebView2 Runtime. Jest zwykle obecny na Windows 11; gdy go brakuje, instalator pobiera oficjalny bootstrapper Microsoft. Sama aplikacja nie wysyła treści schowka ani nie korzysta z sieci.

Wydanie nie ma podpisu Authenticode: w projekcie nie ma certyfikatu wydawcy. Windows może wyświetlić ostrzeżenie o nieznanym wydawcy. Sumy kontrolne są dołączane do wydania jako `SHA256SUMS.txt`; nie zastępują podpisu. Szczegóły: [SIGNING.md](SIGNING.md).

Automatyczne testy obejmują magazyn, migracje, prywatność, obrazy, retencję, szablony, kopie i transformacje. Pełne testy po restarcie systemu, na fizycznie różnych monitorach/DPI i we wszystkich edytorach zewnętrznych pozostają do wykonania. Zbudowany instalator nie oznacza potwierdzenia tych scenariuszy.

## Codzienna praca

- Kopiuj tekst, linki, JSON, kod, kolory, ścieżki, obrazy lub listy plików w innych aplikacjach. Historia reaguje na zmiany schowka.
- **Ctrl+Shift+V** otwiera małe okno przy aktywnej aplikacji. Wpisz frazę, wybierz ↑/↓, naciśnij Enter. Esc zamyka okno. Zakładka Snippety zawęża wyszukiwanie do trwałej biblioteki.
- W głównym oknie: **Ctrl+F** szuka, **Ctrl+N** tworzy snippet. Enter lub dwuklik na liście kopiuje element. Ctrl+C kopiuje wybrany wpis, jeśli nie edytujesz pola ani nie zaznaczyłeś tekstu.
- Snippety mają nazwę, kategorię, opis, tagi, ulubione i opcjonalny globalny skrót. Przykłady można zmieniać lub usuwać; po usunięciu nie wracają przy restarcie.
- W edytorze włącz szablon i wpisz pola, np. `Przeanalizuj {{projekt}} pod kątem {{cel}}`. Przed kopiowaniem lub wklejeniem pojawi się formularz. Powtarzające się pole uzupełniasz raz; wartości nie zmieniają oryginału. Zwykłe snippety nie interpretują nawiasów.
- Edytor pozwala wczytać wcześniejszą wersję snippetu i zapisać ją jako bieżącą. Zachowujemy do 10 wersji na snippet, w łącznym limicie 64 MB.
- **Kopia zapasowa** zapisuje bibliotekę do przenośnego pliku `.scbackup`, szyfrowanego hasłem. Kopia obejmuje aktualne snippety, pola, tagi, kategorie i ulubione; nie obejmuje historii, ustawień ani wcześniejszych wersji. Odtwarzanie dodaje brakujące wpisy, nie nadpisuje istniejących i wyłącza importowane skróty. Bez hasła nie ma odzyskiwania.
- Transformacje pokazują wynik przed kopiowaniem. Oryginał nie jest modyfikowany. JSON zachowuje duże liczby i zapisy wykładnicze. Base64 obsługuje UTF-8.
- Zamknięcie głównego okna pozostawia aplikację w trayu. Dwuklik ikony otwiera okno, menu pozwala wstrzymać historię lub zakończyć proces. Autostart jest opcjonalny i domyślnie wyłączony.

Kopiowanie do schowka nie uruchamia kodu, poleceń terminala ani plików. Przy wklejaniu do aplikacji o wyższych uprawnieniach Windows może zablokować `SendInput`; wtedy treść zostaje skopiowana i pojawia się komunikat o ręcznym Ctrl+V.

## Prywatność i magazyn

Dane znajdują się w `%LOCALAPPDATA%\pl.superclipboard.desktop`:

- `clipboard.db` oraz SQLite WAL: zaszyfrowane DPAPI treści i metadane opisowe;
- `images/*.bin`: osobne, zaszyfrowane obrazy PNG;
- `application.log`: diagnostyka bez treści schowka;
- plik stanu okna zarządzany przez plugin Tauri.

DPAPI wiąże dane z kontem Windows. Skopiowanie folderu na inne konto lub komputer nie stanowi przenośnej kopii zapasowej. Eksport JSON służy do przenoszenia snippetów i zawiera **jawne dane**; przed eksportem aplikacja o tym przypomina. Import jest walidowany, transakcyjny, dodaje nowe snippety i wyłącza ich skróty, aby nie przejmować kombinacji klawiszy. Obsługiwany format: `{ "version": 1, "snippets": [{ "title": "Nazwa", "text": "Treść", "category": "Kategoria", "tags": ["tag"] }] }`.

Indeks pełnotekstowy FTS5 istnieje wyłącznie w pamięci. Jawne pola bazy to techniczne identyfikatory, typ, daty, rozmiar, przypięcie i informacja o bibliotece. Systemowy plik stronicowania lub zrzut pamięci może zawierać odszyfrowane dane. DPAPI nie chroni przed programem działającym na tym samym koncie. Wrażliwa treść może również pozostawać w systemowym schowku niezależnie od aplikacji.

Domyślna lista wyklucza popularne menedżery haseł. Dodaj nazwy własnych procesów `.exe` w ustawieniach. Źródło ustalamy z właściciela schowka; gdy Windows go nie ujawnia, wyświetlamy „nieustalone”. Można całkowicie blokować takie wpisy. Nie rozpoznajemy kart bankowych w przeglądarce: wykluczenie `msedge.exe` lub `chrome.exe` dotyczy całej przeglądarki. Zmiana wykluczeń dotyczy nowych wpisów, nie usuwa wcześniejszej historii.

Respektujemy markery wykluczenia z historii/monitorowania, poufności menedżerów haseł i zakazu przesyłania do chmury. Opcjonalna heurystyka pomija znane prefiksy tokenów, JWT i klucze prywatne. Nie udaje uniwersalnego wykrywacza haseł. Na kopiach tworzonych przez aplikację ustawiamy zakaz historii i chmury Windows.

## Limity i retencja

Domyślnie 2000 nieprzypiętych wpisów, 30 dni i 100 MB zawartości historii. Sprzątanie odbywa się przy zapisie, starcie, zmianie limitów oraz w tle. Przypięte wpisy i snippety nie są usuwane automatycznie, mogą więc zwiększać łączny rozmiar magazynu ponad limit historii. Limit dotyczy logicznego rozmiaru danych, nie ścisłego rozmiaru pliku SQLite; ustawienia pokazują także rzeczywiste zajęcie dysku. SQLite odzyskuje wolne strony, a czyszczenie historii wykonuje VACUUM.

Tekst: do 4 MB UTF-8. Obrazy: do 96 MB surowych danych i 24 MB po kompresji PNG. Biblioteka: do 1000 snippetów i 64 MB treści. Import: do 16 MB. Lista pobierana jest stronami po 80 pozycji; podgląd tekstu wyświetla pierwsze 100 000 znaków, ale kopiuje pełną treść.

Schowek Windows przechowuje tylko bieżącą wartość. Wpisy nadpisane zanim system dostarczy zdarzenie lub podczas przeciążenia nie zawsze można odzyskać. Kolejka zdjęć schowka ma stały limit pamięci; jej przepełnienie jest sygnalizowane. Zablokowany schowek jest odczytywany ponownie przez ograniczony czas.

## Uruchomienie developerskie

Wymagania: Windows x64, Node.js 22+ (testowano 24), Rust MSVC (testowano 1.96), Visual Studio Build Tools z C++ i Windows SDK, WebView2. Zachowaj `package-lock.json` i `src-tauri/Cargo.lock`.

```powershell
npm ci
npm run desktop
```

Sam `npm run dev` udostępnia frontend pod `http://127.0.0.1:1420`, ale zwykła przeglądarka nie zapewnia integracji z Windows. Do testów samego UI służy `http://127.0.0.1:1420/tests/preview.html` z jawną, odrębną fixture testową. Fixture nie jest częścią produkcyjnego `dist` ani instalatora.

## Testy i build produkcyjny

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm audit
npm run release
```

Plik aplikacji: `src-tauri/target/release/super-clipboard.exe`.
Instalator: `src-tauri/target/release/bundle/nsis/Super Clipboard_1.1.0_x64-setup.exe`.
Formatowanie: `npm run format`. Domyślny build nie włącza debugowania WebView.

GitHub Actions wykonuje testy i buduje niepodpisany instalator Windows. Aktualizacje instaluje się ręcznie. Narzędzia budowania pobierają zależności i narzędzia NSIS; te połączenia nie należą do działającej aplikacji. `node scripts/check-source.mjs` sprawdza śledzone pliki pod kątem niedozwolonych danych. `node scripts/licenses.mjs` odtwarza wykaz licencji z zainstalowanych zależności.

Opcjonalny test prawdziwego schowka zastępuje jego bieżącą zawartość danymi syntetycznymi, a na końcu ją czyści. Uruchamiaj świadomie, bez ważnej zawartości w schowku:

```powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml live_windows_clipboard_formats_privacy_and_burst -- --ignored --test-threads=1
```

Przed migracją magazynu v1 do v2 aplikacja tworzy lokalny snapshot `migration-v1.sqlite`. Zachowuje on treści zabezpieczone DPAPI, również po późniejszym czyszczeniu historii; można go usunąć po sprawdzeniu migracji i wykonaniu potrzebnej kopii. Nie otwieraj magazynu v2 starszą aplikacją. Szczegóły przechowywania: [PRIVACY.md](PRIVACY.md), zgłaszanie podatności: [SECURITY.md](SECURITY.md).

## Deinstalacja i odzyskiwanie

Zakończ aplikację z menu trayu i odinstaluj przez Ustawienia Windows → Aplikacje. Deinstalator usuwa również wpis autostartu. Opcja „Usuń również dane aplikacji” usuwa historię i snippety; bez zaznaczenia zachowuje dane na ponowną instalację. Wyeksportuj potrzebne snippety przed usunięciem danych.

W przypadku błędu odczytu DPAPI/bazy aplikacja nie tworzy w jej miejsce pustej historii. Nie usuwaj katalogu danych; zachowaj jego kopię wraz z plikami WAL po zakończeniu procesu. Nie przenoś aktywnej bazy podczas pracy aplikacji.

Szczegóły warstw i przepływu: [ARCHITECTURE.md](ARCHITECTURE.md). Licencje zależności: `THIRD_PARTY_NOTICES.txt`.
