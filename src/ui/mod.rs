use slint::{ModelRc, SharedString, VecModel};
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

slint::include_modules!();

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RomEntry {
    title: String,
    path: PathBuf,
    size: u64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct LibraryConfig {
    roms: Vec<RomEntry>,
}

impl LibraryConfig {
    fn config_path() -> Option<PathBuf> {
        let dir = dirs::config_dir()?.join("chip8-emulator");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir.join("library.json"))
    }

    fn load() -> Self {
        Self::config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("no config dir")?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }
}

fn extract_rom_metadata(path: &Path) -> Option<RomEntry> {
    let metadata = std::fs::metadata(path).ok()?;
    let title = path.file_stem()?.to_string_lossy().to_string();
    Some(RomEntry {
        title,
        path: path.to_path_buf(),
        size: metadata.len(),
    })
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    }
}

pub struct App {
    ui: MainWindow,
    _games_model: Rc<VecModel<GameEntry>>,
}

impl App {
    pub fn new() -> Result<Self, slint::PlatformError> {
        let ui = MainWindow::new()?;

        let mut config = LibraryConfig::load();
        config.roms.retain(|r| r.path.exists());

        let games: Vec<GameEntry> = config
            .roms
            .iter()
            .map(|r| GameEntry {
                title: SharedString::from(r.title.clone()),
                size: SharedString::from(format_size(r.size)),
                icon: slint::Image::default(),
            })
            .collect();

        let games_model = Rc::new(VecModel::from(games));
        ui.set_games(ModelRc::from(games_model.clone()));

        ui.on_extract_rom_path(|transfer| {
            if let Ok(mut paths) = transfer.file_paths()
                && let Some(path) = paths.next()
            {
                let p = path.to_string_lossy();
                if p.ends_with(".ch8") {
                    return SharedString::from(p.to_string());
                }
            }
            SharedString::from("")
        });

        ui.on_rom_dropped({
            let model = games_model.clone();
            let mut cfg = config.clone();
            move |path_str| {
                let path = PathBuf::from(path_str.as_str());
                if let Some(entry) = extract_rom_metadata(&path) {
                    model.push(GameEntry {
                        title: SharedString::from(entry.title.clone()),
                        size: SharedString::from(format_size(entry.size)),
                        icon: slint::Image::default(),
                    });
                    cfg.roms.push(entry);
                    let _ = cfg.save();
                }
            }
        });

        ui.on_browse_rom_clicked({
            let model = games_model.clone();
            let mut cfg = config.clone();
            move || {
                let path = rfd::FileDialog::new()
                    .add_filter("Chip-8 ROM", &["ch8"])
                    .set_title("Select a Chip-8 ROM")
                    .pick_file();

                if let Some(path) = path
                    && let Some(entry) = extract_rom_metadata(&path)
                {
                    model.push(GameEntry {
                        title: SharedString::from(entry.title.clone()),
                        size: SharedString::from(format_size(entry.size)),
                        icon: slint::Image::default(),
                    });
                    cfg.roms.push(entry);
                    let _ = cfg.save();
                }
            }
        });

        ui.on_settings_clicked(|| {
            println!("Settings clicked");
        });

        ui.on_game_selected({
            let ui_weak = ui.as_weak();
            move |_title| {
                ui_weak.unwrap().set_in_game(true);
            }
        });

        ui.on_back_to_launcher({
            let ui_weak = ui.as_weak();
            move || {
                ui_weak.unwrap().set_in_game(false);
            }
        });

        Ok(Self {
            ui,
            _games_model: games_model,
        })
    }

    pub fn run(self) -> Result<(), slint::PlatformError> {
        self.ui.run()
    }
}