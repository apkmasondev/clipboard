mod backup;
mod crypto;
mod model;
mod privacy;
mod store;
mod templates;
mod windows;

use model::*;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub struct AppState {
    store: Mutex<store::Store>,
    shortcuts: Mutex<HashMap<u32, (Shortcut, Option<String>)>>,
    target: Mutex<(isize, u32)>,
    pasting: AtomicBool,
    warnings: Mutex<Vec<String>>,
}
fn locked<'a>(
    s: &'a tauri::State<'_, AppState>,
) -> Result<std::sync::MutexGuard<'a, store::Store>> {
    s.store
        .lock()
        .map_err(|_| "Magazyn danych jest niedostępny. Uruchom ponownie aplikację.".into())
}
fn changed(app: &tauri::AppHandle) {
    let _ = app.emit("changed", ());
}
fn shortcut_plan(
    settings: &Settings,
    entries: &[Entry],
) -> Result<HashMap<u32, (Shortcut, Option<String>)>> {
    let mut map = HashMap::new();
    for (key, id) in std::iter::once((settings.quick_shortcut.as_str(), None)).chain(
        entries
            .iter()
            .filter(|e| e.library && !e.content.shortcut.is_empty())
            .map(|e| (e.content.shortcut.as_str(), Some(e.id.clone()))),
    ) {
        let key: Shortcut = key.parse().map_err(|_| {
            "Nieprawidłowy skrót. Przykład: Ctrl+Shift+V lub Ctrl+Alt+1.".to_string()
        })?;
        if key.mods.is_empty() {
            return Err("Globalny skrót musi zawierać Ctrl, Alt, Shift lub Win.".into());
        }
        if map.insert(key.id(), (key, id)).is_some() {
            return Err("Dwa działania używają tego samego skrótu.".into());
        }
    }
    Ok(map)
}
fn apply_shortcuts(
    app: &tauri::AppHandle,
    plan: HashMap<u32, (Shortcut, Option<String>)>,
) -> Result<()> {
    let state = app.state::<AppState>();
    let old = state
        .shortcuts
        .lock()
        .map_err(|_| "Nie można zmienić skrótów.")?
        .clone();
    let mut added = vec![];
    for (id, (key, _)) in &plan {
        if !old.contains_key(id) {
            if app.global_shortcut().register(*key).is_err() {
                for key in added {
                    let _ = app.global_shortcut().unregister(key);
                }
                return Err(
                    "Skrót jest zajęty przez Windows lub inną aplikację. Wybierz inną kombinację."
                        .into(),
                );
            }
            added.push(*key);
        }
    }
    for (id, (key, _)) in old.iter() {
        if !plan.contains_key(id) {
            let _ = app.global_shortcut().unregister(*key);
        }
    }
    *state
        .shortcuts
        .lock()
        .map_err(|_| "Nie można zmienić skrótów.")? = plan;
    Ok(())
}
fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}
fn show_quick(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    if let Some(w) = app.get_webview_window("quick") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
            return;
        }
        let target = windows::foreground();
        if let Ok(mut t) = state.target.lock() {
            *t = target;
        }
        let _ = windows::position_popup(&w, target.0);
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.emit("quick-open", ());
    }
}
fn do_copy(app: &tauri::AppHandle, id: &str, plain: Option<&str>, paste: bool) -> Result<()> {
    let state = app.state::<AppState>();
    if state.pasting.swap(true, Ordering::SeqCst) {
        return Err("Poprzednie kopiowanie jeszcze trwa.".into());
    }
    let result = (|| {
        if plain.is_some_and(|s| s.len() > MAX_TEXT) {
            return Err("Tekst przekracza 4 MB.".into());
        }
        let (e, image) = {
            let s = locked(&state)?;
            let e = s.get(id)?;
            let image = if e.kind == "image" {
                Some(s.image(id)?)
            } else {
                None
            };
            (e, image)
        };
        if e.content.is_template && plain.is_none() {
            return Err("Ten snippet wymaga uzupełnienia pól. Otwórz formularz szablonu.".into());
        }
        windows::write(&e, image, plain)?;
        if paste {
            if let Some(w) = app.get_webview_window("quick") {
                let _ = w.hide();
            }
            let target = *state
                .target
                .lock()
                .map_err(|_| "Nie można odczytać okna docelowego.")?;
            windows::paste(target)?;
        }
        Ok(())
    })();
    state.pasting.store(false, Ordering::SeqCst);
    result
}
#[tauri::command]
async fn query_entries(
    state: tauri::State<'_, AppState>,
    query: String,
    view: String,
    kind: String,
    sort: String,
    offset: usize,
) -> Result<store::Page> {
    locked(&state)?.query(&query, &view, &kind, &sort, offset)
}
#[tauri::command]
async fn get_entry(state: tauri::State<'_, AppState>, id: String) -> Result<Entry> {
    locked(&state)?.get(&id)
}
#[tauri::command]
async fn template_fields(state: tauri::State<'_, AppState>, id: String) -> Result<Vec<String>> {
    let e = locked(&state)?.get(&id)?;
    if !e.library || !e.content.is_template {
        return Err("To nie jest szablon snippetu.".into());
    }
    templates::fields(&e.content.text)
}
#[tauri::command]
async fn render_template(
    state: tauri::State<'_, AppState>,
    id: String,
    values: std::collections::BTreeMap<String, String>,
) -> Result<String> {
    let e = locked(&state)?.get(&id)?;
    if !e.library || !e.content.is_template {
        return Err("To nie jest szablon snippetu.".into());
    }
    templates::render(&e.content.text, &values)
}
#[tauri::command]
async fn get_revisions(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<store::Revision>> {
    locked(&state)?.revisions(&id)
}
#[tauri::command]
async fn get_revision(
    state: tauri::State<'_, AppState>,
    id: String,
    revision: String,
) -> Result<Content> {
    locked(&state)?.revision(&id, &revision)
}
#[tauri::command]
async fn get_image(state: tauri::State<'_, AppState>, id: String) -> Result<String> {
    use base64::Engine;
    let s = locked(&state)?;
    if s.get(&id)?.kind != "image" {
        return Err("To nie jest obraz.".into());
    }
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(s.image(&id)?)
    ))
}
#[tauri::command]
async fn copy_entry(
    app: tauri::AppHandle,
    id: String,
    plain: Option<String>,
    paste: bool,
) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || do_copy(&app, &id, plain.as_deref(), paste))
        .await
        .map_err(|_| "Nie można wykonać kopiowania.")?
}
#[tauri::command]
async fn pin_entry(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<()> {
    locked(&state)?.pin(&id)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn tag_entry(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    tags: Vec<String>,
) -> Result<()> {
    locked(&state)?.tags(&id, tags)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn delete_entry(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<()> {
    let mut s = locked(&state)?;
    s.delete(&id)?;
    let plan = shortcut_plan(&s.settings, &s.snippets()?)?;
    drop(s);
    apply_shortcuts(&app, plan)?;
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn save_snippet(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: Option<String>,
    content: Content,
    pinned: bool,
) -> Result<Entry> {
    validate_content(&content)?;
    let mut s = locked(&state)?;
    let original = shortcut_plan(&s.settings, &s.snippets()?)?;
    let candidate_id = id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut entries = s.snippets()?;
    entries.retain(|e| e.id != candidate_id);
    entries.push(Entry {
        id: candidate_id.clone(),
        kind: "text".into(),
        created: 0,
        updated: 0,
        bytes: 0,
        pinned,
        library: true,
        content: content.clone(),
    });
    let plan = shortcut_plan(&s.settings, &entries)?;
    apply_shortcuts(&app, plan)?;
    // New snippet IDs are generated by the store; refresh bindings after the write.
    match s.save_snippet(id, content, pinned) {
        Ok(e) => {
            let plan = shortcut_plan(&s.settings, &s.snippets()?)?;
            drop(s);
            apply_shortcuts(&app, plan)?;
            changed(&app);
            Ok(e)
        }
        Err(e) => {
            drop(s);
            let _ = apply_shortcuts(&app, original);
            Err(e)
        }
    }
}
#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<Settings> {
    Ok(locked(&state)?.settings.clone())
}
#[tauri::command]
async fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> Result<()> {
    settings.validate()?;
    let mut s = locked(&state)?;
    let previous = s.settings.clone();
    let entries = s.snippets()?;
    apply_shortcuts(&app, shortcut_plan(&settings, &entries)?)?;
    let result = (|| {
        if settings.autostart != previous.autostart {
            windows::autostart(settings.autostart)?;
        }
        s.save_settings(settings)?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = windows::autostart(previous.autostart);
        let _ = apply_shortcuts(&app, shortcut_plan(&previous, &entries)?);
        return Err(e);
    }
    s.cleanup()?;
    drop(s);
    let _ = app.emit("settings-changed", ());
    changed(&app);
    Ok(())
}
#[tauri::command]
async fn get_stats(state: tauri::State<'_, AppState>) -> Result<store::Stats> {
    locked(&state)?.stats()
}
#[tauri::command]
async fn get_warnings(state: tauri::State<'_, AppState>) -> Result<Vec<String>> {
    Ok(state
        .warnings
        .lock()
        .map_err(|_| "Błąd diagnostyki.")?
        .clone())
}
#[tauri::command]
async fn clear_history(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<usize> {
    let n = locked(&state)?.clear_history()?;
    changed(&app);
    Ok(n)
}
#[tauri::command]
fn open_main(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("quick") {
        let _ = w.hide();
    }
    show_main(&app);
}
#[derive(serde::Serialize, serde::Deserialize)]
struct Export {
    version: u32,
    snippets: Vec<Content>,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct LibraryBackup {
    version: u32,
    snippets: Vec<store::LibraryItem>,
}
fn file_dialog(app: &tauri::AppHandle) -> rfd::FileDialog {
    let dialog = rfd::FileDialog::new();
    match app.get_webview_window("main") {
        Some(window) => dialog.set_parent(&window),
        None => dialog,
    }
}
#[tauri::command]
async fn backup_library(
    app: tauri::AppHandle,
    password: String,
    restore: bool,
) -> Result<Option<usize>> {
    let password = zeroize::Zeroizing::new(password);
    tauri::async_runtime::spawn_blocking(move || {
        if restore {
            let Some(path) = file_dialog(&app)
                .set_title("Odtwórz bibliotekę — szyfrowana kopia")
                .add_filter("Super Clipboard", &["scbackup"])
                .pick_file()
            else {
                return Ok(None);
            };
            let bytes = backup::read_bounded(&path, backup::MAX_BACKUP)?;
            let plaintext = backup::decrypt(&bytes, &password)?;
            let data: LibraryBackup = serde_json::from_slice(&plaintext)
                .map_err(|_| "Kopia nie zawiera poprawnej biblioteki.")?;
            if data.version != 1 {
                return Err("Nieobsługiwana wersja kopii.".into());
            }
            let n = locked(&app.state::<AppState>())?.restore_library(data.snippets)?;
            changed(&app);
            Ok(Some(n))
        } else {
            let data = LibraryBackup {
                version: 1,
                snippets: locked(&app.state::<AppState>())?.library()?,
            };
            let plaintext = zeroize::Zeroizing::new(
                serde_json::to_vec(&data).map_err(|_| "Nie można przygotować biblioteki.")?,
            );
            let bytes = backup::encrypt(&plaintext, &password)?;
            let Some(path) = file_dialog(&app)
                .set_title("Zapisz szyfrowaną kopię biblioteki")
                .add_filter("Super Clipboard", &["scbackup"])
                .set_file_name("super-clipboard-library.scbackup")
                .save_file()
            else {
                return Ok(None);
            };
            backup::write_atomic(&path, &bytes)?;
            Ok(Some(data.snippets.len()))
        }
    })
    .await
    .map_err(|_| "Operacja kopii zapasowej nie powiodła się.")?
}
#[tauri::command]
async fn export_snippets(app: tauri::AppHandle) -> Result<bool> {
    tauri::async_runtime::spawn_blocking(move || {
        let data = {
            let state = app.state::<AppState>();
            let s = locked(&state)?;
            Export {
                version: 1,
                snippets: s
                    .snippets()?
                    .into_iter()
                    .filter(|e| e.library)
                    .map(|e| e.content)
                    .collect(),
            }
        };
        let Some(path) = file_dialog(&app)
            .set_title("Eksport snippetów — jawny plik JSON")
            .add_filter("JSON", &["json"])
            .set_file_name("super-clipboard-snippets.json")
            .save_file()
        else {
            return Ok(false);
        };
        backup::write_atomic(
            &path,
            &serde_json::to_vec_pretty(&data).map_err(|_| "Nie można przygotować eksportu.")?,
        )?;
        Ok(true)
    })
    .await
    .map_err(|_| "Eksport nie powiódł się.")?
}
#[tauri::command]
async fn import_snippets(app: tauri::AppHandle) -> Result<usize> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = file_dialog(&app)
            .set_title("Import snippetów")
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return Ok(0);
        };
        if std::fs::metadata(&path)
            .map_err(|_| "Nie można odczytać pliku.")?
            .len()
            > 16 * 1024 * 1024
        {
            return Err("Maksymalny rozmiar importu to 16 MB.".into());
        }
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| "Nie można otworzyć importu.")?
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Nie można odczytać importu.")?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err("Maksymalny rozmiar importu to 16 MB.".into());
        }
        let data: Export = serde_json::from_slice(&bytes)
            .map_err(|_| "Niepoprawny plik eksportu Super Clipboard.")?;
        if data.version != 1 {
            return Err("Nieobsługiwana wersja eksportu.".into());
        }
        let state = app.state::<AppState>();
        let n = locked(&state)?.import(data.snippets)?;
        changed(&app);
        Ok(n)
    })
    .await
    .map_err(|_| "Import nie powiódł się.")?
}
fn seed(store: &mut store::Store) -> Result<()> {
    for (title,category,tags,text) in [
        ("Publikacja na GitHub Pages","Publikacja",vec!["github","bezpieczeństwo"],"Opublikuj projekt na GitHub Pages. Do repozytorium nie dodawaj żadnych sekretów, kluczy, prywatnych plików, planów projektu, plików roboczych ani materiałów źródłowych. Repozytorium ma zawierać wyłącznie pliki wymagane do działania wersji produkcyjnej, gotowy katalog dystrybucyjny oraz README."),
        ("Przegląd kodu","Praca z AI",vec!["review"],"Przeanalizuj kod pod kątem błędów, bezpieczeństwa, przypadków brzegowych i czytelności. Dla każdej uwagi podaj lokalizację, konkretny scenariusz błędu, jego wpływ i propozycję poprawki. Odróżnij fakty od przypuszczeń. Nie zmieniaj plików przed zakończeniem analizy."),
        ("Plan przed implementacją","Praca z AI",vec!["planowanie"],"Najpierw sprawdź istniejącą architekturę i wymagania. Opisz proponowane zmiany, ryzyka oraz sposób weryfikacji. Zachowaj istniejące konwencje projektu. Zadawaj pytania tylko wtedy, gdy brakująca informacja blokuje poprawne wykonanie zadania."),
        ("Checklista wydania","Publikacja",vec!["release"],"[ ] Testy i build produkcyjny przechodzą\n[ ] Brak sekretów i danych prywatnych\n[ ] Zależności sprawdzone\n[ ] README i instrukcja instalacji aktualne\n[ ] Migracje oraz rollback sprawdzone\n[ ] Kluczowe ścieżki użytkownika przetestowane\n[ ] Znane ograniczenia opisane"),
        ("Podsumowanie zmian","Wiadomości",vec!["komunikacja"],"Zakończone: \n\nNajważniejsze zmiany: \n\nSposób weryfikacji: \n\nOgraniczenia i dalsze kroki: "),
        ("Stan repozytorium","Terminal",vec!["git"],"git status --short")
    ] { store.save_snippet(None,Content{title:title.into(),category:category.into(),tags:tags.into_iter().map(String::from).collect(),text:text.into(),description:"Przykład startowy. Możesz go dowolnie edytować lub usunąć.".into(),..Default::default()},false)?; }
    Ok(())
}
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main(app)
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&["quick"])
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED,
                )
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, key, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let state = app.state::<AppState>();
                    let action = state
                        .shortcuts
                        .lock()
                        .ok()
                        .and_then(|m| m.get(&key.id()).cloned());
                    match action {
                        Some((_, Some(id))) => {
                            let is_template = state
                                .store
                                .lock()
                                .ok()
                                .and_then(|s| s.get(&id).ok())
                                .is_some_and(|e| e.content.is_template);
                            if is_template {
                                if let Some(w) = app.get_webview_window("quick") {
                                    if !w.is_visible().unwrap_or(false) {
                                        show_quick(app);
                                    }
                                    let _ = w.emit("template-open", &id);
                                }
                                return;
                            }
                            if let Ok(mut t) = state.target.lock() {
                                *t = windows::foreground();
                            }
                            let app = app.clone();
                            std::thread::spawn(move || {
                                if let Err(e) = do_copy(&app, &id, None, true) {
                                    let _ = app.emit("notice", &e);
                                    show_main(&app);
                                }
                            });
                        }
                        Some((_, None)) => show_quick(app),
                        None => {}
                    }
                })
                .build(),
        )
        .setup(|app| {
            let root = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&root)?;
            let logfile = root.join("application.log");
            if std::fs::metadata(&logfile).is_ok_and(|m| m.len() > 1024 * 1024) {
                let _ = std::fs::rename(&logfile, root.join("application.previous.log"));
            }
            if let Ok(file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(logfile)
            {
                let _ = simplelog::WriteLogger::init(
                    log::LevelFilter::Warn,
                    simplelog::Config::default(),
                    file,
                );
            }
            let first = !root.join("clipboard.db").exists();
            let mut store = store::Store::open(root).map_err(std::io::Error::other)?;
            if first && store.is_empty().map_err(std::io::Error::other)? {
                seed(&mut store).map_err(std::io::Error::other)?;
            }
            let plan = shortcut_plan(
                &store.settings,
                &store.snippets().map_err(std::io::Error::other)?,
            )
            .map_err(std::io::Error::other)?;
            app.manage(AppState {
                store: Mutex::new(store),
                shortcuts: Mutex::new(HashMap::new()),
                target: Mutex::new((0, 0)),
                pasting: AtomicBool::new(false),
                warnings: Mutex::new(vec![]),
            });
            if let Err(e) = apply_shortcuts(app.handle(), plan) {
                app.state::<AppState>()
                    .warnings
                    .lock()
                    .map_err(|_| std::io::Error::other("Shortcut lock"))?
                    .push(e);
            }
            windows::start_listener(app.handle().clone()).map_err(std::io::Error::other)?;
            use tauri::menu::{Menu, MenuItem};
            let open =
                MenuItem::with_id(app, "open", "Otwórz Super Clipboard", true, None::<&str>)?;
            let pause = MenuItem::with_id(
                app,
                "pause",
                "Wstrzymaj / wznów historię",
                true,
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "quit", "Zakończ", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &pause, &quit])?;
            tauri::tray::TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .ok_or("Brak ikony aplikacji")?
                        .clone(),
                )
                .tooltip("Super Clipboard · Ctrl+Shift+V")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "open" => show_main(app),
                    "pause" => {
                        let state = app.state::<AppState>();
                        if let Ok(mut s) = state.store.lock() {
                            let mut settings = s.settings.clone();
                            settings.paused = !settings.paused;
                            if s.save_settings(settings).is_ok() {
                                let _ = app.emit("settings-changed", ());
                            }
                        };
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, e| {
                    if matches!(e, tauri::tray::TrayIconEvent::DoubleClick { .. }) {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            if !std::env::args().any(|s| s == "--background") {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::Focused(false) if window.label() == "quick" => {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            query_entries,
            get_entry,
            template_fields,
            render_template,
            get_revisions,
            get_revision,
            backup_library,
            get_image,
            copy_entry,
            pin_entry,
            tag_entry,
            delete_entry,
            save_snippet,
            get_settings,
            save_settings,
            get_stats,
            get_warnings,
            clear_history,
            open_main,
            export_snippets,
            import_snippets
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        log::error!("Application startup failed: {e}");
        rfd::MessageDialog::new()
            .set_title("Super Clipboard — nie można uruchomić")
            .set_description(format!(
                "{e}\n\nNie usuwaj bazy danych. Sprawdź uprawnienia i dostępne miejsce na dysku."
            ))
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
}
