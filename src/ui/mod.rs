use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{Color, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use rodio::{OutputStream, Sink};

slint::include_modules!();

use crate::core::cpu::Cpu;

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
    if !metadata.is_file() {
        return None;
    }
    let title = path.file_stem()?.to_string_lossy().to_string();
    if title.is_empty() {
        return None;
    }
    Some(RomEntry {
        title,
        path: path.to_path_buf(),
        size: metadata.len(),
    })
}

fn is_valid_rom_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ch8") || ext.eq_ignore_ascii_case("rom"))
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
    buf: &[bool; 64 * 32],
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
            let (r, g, b) = if buf[py * 64 + px] {
                (fg[0], fg[1], fg[2])
            } else {
                (bg[0], bg[1], bg[2])
            };
            slice[(y * w + x) as usize] = Rgba8Pixel { r, g, b, a: 255 };
        }
    }
    slint::Image::from_rgba8(pb)
}

fn map_key(key: &str) -> Option<usize> {
    match &key.to_lowercase()[..] {
        "1" => Some(0x1),
        "2" => Some(0x2),
        "3" => Some(0x3),
        "4" => Some(0xC),
        // en
        "q" => Some(0x4),
        "w" => Some(0x5),
        "e" => Some(0x6),
        "r" => Some(0xD),
        "a" => Some(0x7),
        "s" => Some(0x8),
        "d" => Some(0x9),
        "f" => Some(0xE),
        "z" => Some(0xA),
        "x" => Some(0x0),
        "c" => Some(0xB),
        "v" => Some(0xF),
        // ru
        "й" => Some(0x4),
        "ц" => Some(0x5),
        "у" => Some(0x6),
        "к" => Some(0xD),
        "ф" => Some(0x7),
        "ы" => Some(0x8),
        "в" => Some(0x9),
        "а" => Some(0xE),
        "я" => Some(0xA),
        "ч" => Some(0x0),
        "с" => Some(0xB),
        "м" => Some(0xF),
        _ => None,
    }
}

pub struct App {
    ui: MainWindow,
    _games_model: Rc<VecModel<GameEntry>>,
    _timer: slint::Timer,
    _stream: OutputStream,
}

impl App {
    pub fn new() -> Result<Self, slint::PlatformError> {
        let ui = MainWindow::new()?;

        let chip8 = Arc::new(Mutex::new(Cpu::new()));
        let is_running = Arc::new(AtomicBool::new(false));

        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Rc::new(RefCell::new(Sink::try_new(&stream_handle).unwrap()));

        sink.borrow_mut()
            .append(rodio::source::SineWave::new(440.0));
        sink.borrow_mut().pause();

        let config = Rc::new(RefCell::new(LibraryConfig::load()));
        config.borrow_mut().roms.retain(|r| r.path.exists());

        let games: Vec<GameEntry> = config
            .borrow()
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

        ui.set_settings_clock(config.borrow().settings.clock_frequency as i32);
        ui.set_settings_fg(color_from_rgb(config.borrow().settings.fg_color));
        ui.set_settings_bg(color_from_rgb(config.borrow().settings.bg_color));

        let shared_settings = Rc::new(RefCell::new(config.borrow().settings.clone()));

        ui.window().on_winit_window_event({
            let ui_weak = ui.as_weak();
            let games_model = games_model.clone();
            let config = config.clone();
            move |_window, event| {
                let Some(ui) = ui_weak.upgrade() else {
                    return EventResult::Propagate;
                };

                match event {
                    WindowEvent::HoveredFile(_) => {
                        ui.set_drag_hovered(true);
                    }
                    WindowEvent::HoveredFileCancelled => {
                        ui.set_drag_hovered(false);
                    }
                    WindowEvent::DroppedFile(path) => {
                        ui.set_drag_hovered(false);
                        if !ui.get_in_game()
                            && !ui.get_show_settings()
                            && is_valid_rom_extension(path)
                            && !config.borrow().roms.iter().any(|r| r.path == *path)
                        {
                            match extract_rom_metadata(path) {
                                Some(entry) => {
                                    games_model.push(GameEntry {
                                        title: SharedString::from(entry.title.clone()),
                                        size: SharedString::from(format_size(entry.size)),
                                        icon: slint::Image::default(),
                                    });
                                    config.borrow_mut().roms.push(entry);
                                    let _ = config.borrow().save();
                                }
                                None => {
                                    eprintln!(
                                        "[DnD] Error: file wasn't found on the path {:?}",
                                        path
                                    );
                                }
                            }
                        }
                    }
                    _ => {}
                }

                EventResult::Propagate
            }
        });

        ui.on_browse_rom_clicked({
            let model = games_model.clone();
            let cfg = config.clone();
            move || {
                let path = rfd::FileDialog::new()
                    .add_filter("Chip-8 ROM", &["ch8"])
                    .set_title("Select a Chip-8 ROM")
                    .pick_file();

                if let Some(path) = path
                    && let Some(entry) = extract_rom_metadata(&path)
                    && !cfg.borrow().roms.iter().any(|r| r.path == path)
                {
                    model.push(GameEntry {
                        title: SharedString::from(entry.title.clone()),
                        size: SharedString::from(format_size(entry.size)),
                        icon: slint::Image::default(),
                    });
                    cfg.borrow_mut().roms.push(entry);
                    let _ = cfg.borrow().save();
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
            let cfg = config.clone();
            move |freq, fg, bg| {
                let clamped = freq.clamp(200, 1200);
                let fg_rgb = rgb_from_color(fg);
                let bg_rgb = rgb_from_color(bg);

                cfg.borrow_mut().settings.clock_frequency = clamped as u32;
                cfg.borrow_mut().settings.fg_color = fg_rgb;
                cfg.borrow_mut().settings.bg_color = bg_rgb;
                let _ = cfg.borrow().save();

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
            let clone_config = config.clone();
            let clone_settings = shared_settings.clone();
            let clone_cpu = Arc::clone(&chip8);
            let current_state = Arc::clone(&is_running);

            let ui_weak = ui.as_weak();

            move |_title| {
                current_state.store(true, Ordering::Relaxed);
                let clone_is_running = Arc::clone(&current_state);

                ui_weak.unwrap().set_in_game(true);

                let config_binding = clone_config.borrow();
                let game_entry = config_binding.roms.iter().find(|rom| _title == rom.title);

                if let Some(entry) = game_entry
                    && let Ok(rom_bytes) = std::fs::read(&entry.path)
                {
                    let mut locked_cpu = clone_cpu.lock().unwrap();

                    locked_cpu.load_rom(&rom_bytes).unwrap();
                }

                let locked_cpu = Arc::clone(&clone_cpu);

                let clock_speed = clone_settings.borrow().clock_frequency;

                std::thread::spawn(move || {
                    while clone_is_running.load(Ordering::Relaxed) {
                        {
                            let mut cpu = locked_cpu.lock().unwrap();

                            cpu.tick().unwrap();
                        }

                        std::thread::sleep(std::time::Duration::from_micros(
                            1_000_000 / (clock_speed as u64),
                        ));
                    }
                });
            }
        });

        ui.on_back_to_launcher({
            let current_state = Arc::clone(&is_running);
            let cpu = Arc::clone(&chip8);

            let ui_weak = ui.as_weak();
            move || {
                let mut locked_cpu = cpu.lock().unwrap();
                locked_cpu.clear();

                current_state.store(false, Ordering::Relaxed);

                ui_weak.unwrap().set_in_game(false);
            }
        });

        ui.on_press_key({
            let cpu = Arc::clone(&chip8);

            move |_key| {
                let mut locked_cpu = cpu.lock().unwrap();

                if let Some(key) = map_key(_key.as_str()) {
                    locked_cpu.keypad[key] = true;
                }
            }
        });

        ui.on_release_key({
            let cpu = Arc::clone(&chip8);

            move |_key| {
                let mut locked_cpu = cpu.lock().unwrap();

                if let Some(key) = map_key(_key.as_str()) {
                    locked_cpu.keypad[key] = false;
                }
            }
        });

        let ui_weak = ui.as_weak();
        let shared_for_timer = shared_settings.clone();
        let timer = slint::Timer::default();
        let cpu = Arc::clone(&chip8);
        let mut is_playing = false;

        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(16),
            move || {
                let mut locked_cpu = cpu.lock().unwrap();
                let buf = locked_cpu.display;
                locked_cpu.update_timers();

                let should_play = locked_cpu.sound_timer > 0;

                if should_play != is_playing {
                    if should_play {
                        sink.borrow_mut().play();
                        is_playing = true;
                    } else {
                        sink.borrow_mut().pause();
                        is_playing = false;
                    }
                }

                let s = shared_for_timer.borrow().clone();
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
            _stream: stream,
        })
    }

    pub fn run(self) -> Result<(), slint::PlatformError> {
        self.ui.run()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_is_valid_rom_extension() {
        assert!(is_valid_rom_extension(Path::new("game.ch8")));
        assert!(is_valid_rom_extension(Path::new("GAME.CH8")));
        assert!(is_valid_rom_extension(Path::new("game.rom")));
        assert!(is_valid_rom_extension(Path::new("GAME.ROM")));
        assert!(is_valid_rom_extension(Path::new("/path/to/game.ch8")));

        assert!(!is_valid_rom_extension(Path::new("game.txt")));
        assert!(!is_valid_rom_extension(Path::new("game.bin")));
        assert!(!is_valid_rom_extension(Path::new("game")));
        assert!(!is_valid_rom_extension(Path::new("")));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(2048), "2.0 KB");
    }

    #[test]
    fn test_extract_rom_metadata() {
        let nonexistent = Path::new("nonexistent_file_12345.ch8");
        assert!(extract_rom_metadata(nonexistent).is_none());

        let temp_dir = std::env::temp_dir();
        // A directory should not be accepted as a ROM file
        assert!(extract_rom_metadata(&temp_dir).is_none());

        let temp_file_path = temp_dir.join("test_pong_game.ch8");
        {
            let mut file = File::create(&temp_file_path).unwrap();
            file.write_all(b"CHIP8").unwrap();
        }

        let entry = extract_rom_metadata(&temp_file_path).expect("Should extract metadata");
        assert_eq!(entry.title, "test_pong_game");
        assert_eq!(entry.size, 5);
        assert_eq!(entry.path, temp_file_path);

        let _ = std::fs::remove_file(&temp_file_path);
    }
}
