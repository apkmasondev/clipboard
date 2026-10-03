# Super Clipboard — architektura i UX

## Decyzje
Windows 11 x64, Tauri 2 / WebView2, React i TypeScript, Rust, SQLite. Brak serwera, telemetrii, AI, zdalnych fontów ani automatycznego sprawdzania aktualizacji. Windows wymaga zainstalowanego WebView2; instalator może pobrać runtime Microsoft, jeśli go brakuje.

## Przepływ danych
Natywne WM_CLIPBOARDUPDATE → odczyt z ograniczonym retry → polityka prywatności → klasyfikacja → deduplikacja → DPAPI → SQLite. Treści oraz metadane opisowe są zaszyfrowane dla bieżącego użytkownika Windows. Obrazy PNG przechowywane osobno, również zaszyfrowane DPAPI. Nietrwały indeks FTS5 istnieje wyłącznie w pamięci. SQLite przechowuje jawne techniczne identyfikatory, typy, czasy, rozmiary i przypięcie. To nie jest ochrona przed złośliwym oprogramowaniem działającym na tym samym koncie Windows.

Warstwy Rust: model i walidacja, polityka prywatności, storage/migracje/retencja, integracja Windows, komendy aplikacji. Frontend: typowany adapter IPC, nawigacja, lista, podgląd, edytor snippetów, ustawienia, osobne okno szybkiego wyboru. Żadne treści HTML/RTF nie są renderowane jako aktywny HTML.

## UX
Podglądy obrazów: `get_thumbnail` działa w wątku roboczym, odczytuje zaszyfrowany oryginał przez magazyn i skaluje PNG do maksymalnie 512 × 320, zachowując proporcje i alfa. Wspólna blokada dekodera ogranicza szczytowe zużycie pamięci; blokada bazy nie obejmuje skalowania. Przed zwrotem ponownie sprawdzamy istnienie wpisu, ponieważ retencja lub użytkownik mogą go usunąć w trakcie pracy. Nie ma plików miniatur ani migracji bazy. Wymiary w Summary pochodzą z istniejących zaszyfrowanych metadanych.

`PreviewQueue` współdzieli żądania listy i większego podglądu, ogranicza równoległość do dwóch IPC na okno, pomija anulowane zadania oczekujące i przechowuje do 32 wyników / 4 MiB ciągów UTF-16. Generacja cache zapobiega odtworzeniu starego cache przez żądanie rozpoczęte przed jego wyczyszczeniem. `ImagePreview` obserwuje przecięcie z widocznym obszarem i widoczność dokumentu; nie zachowuje obrazu po wyjściu wiersza poza ekran. Zmiana ID nie może pokazać wyniku wcześniejszego żądania. Błąd jednego podglądu nie zatrzymuje pozostałych. Kod kopiowania i wklejania nadal korzysta z oryginalnego PNG.

Stały lewy pasek nawigacji; lista z wyszukiwaniem, filtrem typu i sortowaniem; podgląd z działaniami. Dwa motywy i ustawienie systemowe. Jeden kolor akcentu, wyraźne obramowania focus, tekst przy statusach. Historia i trwałe snippety mają odrębny cykl życia. Przypięte wpisy są chronione przed retencją; ich rozmiar jest widoczny i może przekroczyć limit historii. Usuwanie i czyszczenie wymagają potwierdzenia.

Popup Ctrl+Shift+V: zapis docelowego HWND, pozycjonowanie na monitorze aktywnego okna, wyszukiwanie, strzałki, Enter, Esc. Wklejanie przywraca docelowe okno i wysyła Ctrl+V dopiero po zwolnieniu klawiszy fizycznych. Nie wkleja do przypadkowego okna, jeśli nie można odzyskać fokusu. Windows UIPI może blokować wklejanie do procesów administratora; dostępne jest ręczne Ctrl+V.

Aktywacja okna między kolejkami wejścia jest asynchroniczna, dlatego przed wklejeniem czekamy do 300 ms na potwierdzenie właściwego HWND, bez powtarzanego przejmowania fokusu. Zachowanie API opisuje [dokumentacja Microsoft SetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow).

## Ograniczenia i bezpieczeństwo
Wykluczenia są nazwami procesów .exe, a nie tytułami okien. Źródło pochodzi z właściciela schowka, nie zgadywania po aktywnym oknie. Nieznane źródła można blokować. Respektowane formaty poufności Windows i menedżerów haseł. Wykrywanie sekretów dotyczy rozpoznawalnych prefiksów i kluczy prywatnych, nie zwykłych losowych słów. Brak udawanej detekcji każdego hasła.

Eksport jest świadomą operacją użytkownika i zawiera jawne snippety. Import jest limitowany, walidowany i transakcyjny. Skróty importowanych snippetów są wyłączone. Nazwy plików magazynu pochodzą wyłącznie z UUID. Treści schowka nie trafiają do logów. Retencja usuwa nieprzypięte wpisy historii, a nie bibliotekę snippetów.

## Biblioteka, migracje i kopie

Migracja v2 dodaje szyfrowane DPAPI rewizje treści snippetów. Zapis nowej wersji i poprzedniej treści jest jedną transakcją, a usunięcie wpisu usuwa też rewizje przez klucz obcy. Limit wynosi 10 rewizji na wpis i 64 MB globalnie. Przed migracją v1 powstaje spójny snapshot SQLite przez VACUUM INTO. Inicjalizacja indeksu odszyfrowuje wpisy pojedynczo.

Moduł `templates` parsuje jawnie włączone pola `{{nazwa}}`: do 20 unikalnych pól i 200 wystąpień. Zastąpienie jest jednokrotne; wartości nie są wykonywane, nie odwołują się do systemu i nie są interpretowane rekurencyjnie. Ten sam parser i renderer obsługuje główne okno, popup i globalne skróty.

Moduł `backup` szyfruje aktualną bibliotekę przy użyciu AES-256-GCM oraz Argon2id (64 MiB, 3 iteracje, równoległość 1). Każdy plik ma losową sól i nonce; nagłówek podlega uwierzytelnieniu. Hasło i bufor odszyfrowany są czyszczone przez Zeroizing w Rust; nie stanowi to gwarancji wyzerowania kopii w JavaScript/WebView. Rozmiar pliku jest ograniczony do 80 MB, zapis korzysta z tymczasowego pliku w tym samym katalogu i atomowego zastąpienia. KDF i okna plików działają poza wątkiem interfejsu. Odtwarzanie waliduje całą bibliotekę przed transakcyjnym importem, zachowuje przypięcia, pomija identyczne treści i usuwa skróty z importu.
