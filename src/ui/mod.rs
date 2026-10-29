use slint::{Color, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

slint::include_modules!();

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RomEntry {
    title: String,
    path: PathBuf,
    size: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct EmulatorSettings {
    clock_frequency: u32,
    fg_color: [u8; 3],
    bg_color: [u8; 3],
}

impl Default for EmulatorSettings {
    fn default() -> Self {
        Self {
            clock_frequency: 700,
            fg_color: [0xcd, 0xd6, 0xf4],
            bg_color: [0x1e, 0x1e, 0x2e],
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct LibraryConfig {
    roms: Vec<RomEntry>,
    #[serde(default)]
    settings: EmulatorSettings,
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

fn color_from_rgb(rgb: [u8; 3]) -> Color {
    Color::from_rgb_u8(rgb[0], rgb[1], rgb[2])
}

fn rgb_from_color(c: Color) -> [u8; 3] {
    [c.red(), c.green(), c.blue()]
}

fn convert_framebuffer(
    buf: &[[bool; 64]; 32],
    fg: [u8; 3],
    bg: [u8; 3],
    scale: u32,
) -> slint::Image {
    let w = 64 * scale;
    let h = 32 * scale;
    let mut pb = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    let slice = pb.make_mut_slice();

    for y in 0..h {
        for x in 0..w {
            let px = (x / scale) as usize;
            let py = (y / scale) as usize;
            let (r, g, b) = if buf[py][px] {
                (fg[0], fg[1], fg[2])
            } else {
                (bg[0], bg[1], bg[2])
            };
            slice[(y * w + x) as usize] = Rgba8Pixel { r, g, b, a: 255 };
        }
    }
    slint::Image::from_rgba8(pb)
}

fn dummy_framebuffer() -> [[bool; 64]; 32] {
    let mut buf = [[false; 64]; 32];

    for cell in buf[0].iter_mut() {
        *cell = true;
    }
    for cell in buf[31].iter_mut() {
        *cell = true;
    }
    for row in buf.iter_mut() {
        row[0] = true;
        row[63] = true;
    }

    buf
}

pub struct App {
    ui: MainWindow,
    _games_model: Rc<VecModel<GameEntry>>,
    _timer: slint::Timer,
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

        ui.set_settings_clock(config.settings.clock_frequency as i32);
        ui.set_settings_fg(color_from_rgb(config.settings.fg_color));
        ui.set_settings_bg(color_from_rgb(config.settings.bg_color));

        let shared_settings = Rc::new(RefCell::new(config.settings.clone()));

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

        ui.on_settings_clicked({
            let ui_weak = ui.as_weak();
            let shared = shared_settings.clone();
            move || {
                // Перед открытием модалки синхронизируем её свойства
                // из актуальных настроек, чтобы слайдеры показывали верные значения
                if let Some(ui) = ui_weak.upgrade() {
                    let s = shared.borrow();
                    ui.set_settings_clock(s.clock_frequency as i32);
                    ui.set_settings_fg(color_from_rgb(s.fg_color));
                    ui.set_settings_bg(color_from_rgb(s.bg_color));
                    ui.set_show_settings(true);
                }
            }
        });

        ui.on_save_settings({
            let ui_weak = ui.as_weak();
            let shared = shared_settings.clone();
            let mut cfg = config.clone();
            move |freq, fg, bg| {
                let clamped = freq.clamp(200, 1200);
                let fg_rgb = rgb_from_color(fg);
                let bg_rgb = rgb_from_color(bg);

                cfg.settings.clock_frequency = clamped as u32;
                cfg.settings.fg_color = fg_rgb;
                cfg.settings.bg_color = bg_rgb;
                let _ = cfg.save();

                let mut s = shared.borrow_mut();
                s.clock_frequency = clamped as u32;
                s.fg_color = fg_rgb;
                s.bg_color = bg_rgb;
                drop(s);

                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_settings_clock(clamped);
                    ui.set_settings_fg(fg);
                    ui.set_settings_bg(bg);
                }
            }
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

        let ui_weak = ui.as_weak();
        let shared_for_timer = shared_settings.clone();
        let timer = slint::Timer::default();
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(16),
            move || {
                let s = shared_for_timer.borrow().clone();
                let buf = dummy_framebuffer();
                let image = convert_framebuffer(&buf, s.fg_color, s.bg_color, 4);
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_screen(image);
                }
            },
        );

        Ok(Self {
            ui,
            _games_model: games_model,
            _timer: timer,
        })
    }

    pub fn run(self) -> Result<(), slint::PlatformError> {
        self.ui.run()
    }
}
