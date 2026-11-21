use rand::seq::SliceRandom;
use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{Color, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
#[serde(default)]
struct EmulatorSettings {
    clock_frequency: u32,
    fg_color: [u8; 3],
    bg_color: [u8; 3],
    show_icons: bool,
    show_all: bool,
}

impl Default for EmulatorSettings {
    fn default() -> Self {
        Self {
            clock_frequency: 700,
            fg_color: [0xcd, 0xd6, 0xf4],
            bg_color: [0x1e, 0x1e, 0x2e],
            show_icons: true,
            show_all: false,
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

fn random_cartridge_color() -> Color {
    const COLORS: [[u8; 3]; 7] = [
        [0x89, 0xb4, 0xfa], // Blue
        [0xf3, 0x8b, 0xa8], // Red
        [0xa6, 0xe3, 0xa1], // Green
        [0xfa, 0xb3, 0x87], // Peach
        [0xcb, 0xa6, 0xf7], // Mauve
        [0xf9, 0xe2, 0xaf], // Yellow
        [0x94, 0xe2, 0xd5], // Teal
    ];

    let mut rng = rand::thread_rng();
    let [r, g, b] = *COLORS.choose(&mut rng).unwrap();

    Color::from_rgb_u8(r, g, b)
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

#[rustfmt::skip]
fn map_key(key: &str) -> Option<usize> {
    match &key.to_lowercase()[..] {
        "1" => Some(0x1),
        "2" => Some(0x2),
        "3" => Some(0x3),
        "4" => Some(0xC),

      // en | ru
        "q" | "й" => Some(0x4),
        "w" | "ц" => Some(0x5),
        "e" | "у" => Some(0x6),
        "r" | "к" => Some(0xD),
        "a" | "ф" => Some(0x7),
        "s" | "ы" => Some(0x8),
        "d" | "в" => Some(0x9),
        "f" | "а" => Some(0xE),
        "z" | "я" => Some(0xA),
        "x" | "ч" => Some(0x0),
        "c" | "с" => Some(0xB),
        "v" | "м" => Some(0xF),

        _ => None,
    }
}

struct PreviewInstance {
    cpu: Cpu,
    halted: bool,
}

impl PreviewInstance {
    fn from_path(path: &Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        let mut cpu = Cpu::new();
        let halted = cpu.load_rom(&bytes).is_err();
        Some(Self { cpu, halted })
    }
}

pub struct EmulatorState {
    active_cpu: Cpu,
    is_running: bool,
    sink: Sink,
    previews: Vec<Option<PreviewInstance>>,
    hovered_index: Option<usize>,
}

impl EmulatorState {
    pub fn new(stream_handle: &rodio::OutputStreamHandle, num_games: usize) -> Self {
        let sink = Sink::try_new(stream_handle).unwrap();

        sink.append(rodio::source::SineWave::new(440.0));
        sink.pause();

        Self {
            active_cpu: Cpu::new(),
            is_running: false,
            sink,
            previews: (0..num_games).map(|_| None).collect(),
            hovered_index: None,
        }
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

        let (stream, stream_handle) = OutputStream::try_default().unwrap();

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
                card_color: random_cartridge_color(),
            })
            .collect();

        let emulator_state = Rc::new(RefCell::new(EmulatorState::new(
            &stream_handle,
            games.len(),
        )));

        let games_model = Rc::new(VecModel::from(games));
        ui.set_games(ModelRc::from(games_model.clone()));

        ui.set_settings_clock(config.borrow().settings.clock_frequency as i32);
        ui.set_settings_fg(color_from_rgb(config.borrow().settings.fg_color));
        ui.set_settings_bg(color_from_rgb(config.borrow().settings.bg_color));
        ui.set_show_icons(config.borrow().settings.show_icons);
        ui.set_show_all(config.borrow().settings.show_all);

        let shared_settings = Rc::new(RefCell::new(config.borrow().settings.clone()));

        ui.window().on_winit_window_event({
            let ui_weak = ui.as_weak();
            let games_model = games_model.clone();
            let config = config.clone();
            let state = emulator_state.clone();

            move |_window, event| {
                let Some(ui) = ui_weak.upgrade() else {
                    return EventResult::Propagate;
                };

                match event {
                    WindowEvent::HoveredFile(_) => {
                        if !ui.get_in_game() && !ui.get_show_settings() {
                            ui.set_drag_hovered(true);
                        }
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
                                        card_color: random_cartridge_color(),
                                    });
                                    config.borrow_mut().roms.push(entry);
                                    let _ = config.borrow().save();
                                    state.borrow_mut().previews.push(None);
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
            let state = emulator_state.clone();

            move || {
                let path = rfd::FileDialog::new()
                    .add_filter("Chip-8 ROM", &["ch8", "rom"])
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
                        card_color: random_cartridge_color(),
                    });
                    cfg.borrow_mut().roms.push(entry);
                    let _ = cfg.borrow().save();
                    state.borrow_mut().previews.push(None);
                }
            }
        });

        ui.on_settings_clicked({
            let ui_weak = ui.as_weak();
            let shared = shared_settings.clone();

            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    let s = shared.borrow();
                    ui.set_settings_clock(s.clock_frequency as i32);
                    ui.set_settings_fg(color_from_rgb(s.fg_color));
                    ui.set_settings_bg(color_from_rgb(s.bg_color));
                    ui.set_show_icons(s.show_icons);
                    ui.set_show_all(s.show_all);
                    ui.set_show_settings(true);
                }
            }
        });

        ui.on_save_settings({
            let ui_weak = ui.as_weak();
            let shared = shared_settings.clone();
            let cfg = config.clone();

            move |freq, fg, bg, si, sa| {
                let clamped = freq.clamp(200, 1200);
                let fg_rgb = rgb_from_color(fg);
                let bg_rgb = rgb_from_color(bg);

                {
                    cfg.borrow_mut().settings.clock_frequency = clamped as u32;
                    cfg.borrow_mut().settings.fg_color = fg_rgb;
                    cfg.borrow_mut().settings.bg_color = bg_rgb;
                    cfg.borrow_mut().settings.show_icons = si;
                    cfg.borrow_mut().settings.show_all = sa;
                    let _ = cfg.borrow().save();
                }

                {
                    let mut s = shared.borrow_mut();
                    s.clock_frequency = clamped as u32;
                    s.fg_color = fg_rgb;
                    s.bg_color = bg_rgb;
                    s.show_icons = si;
                    s.show_all = sa;
                }

                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_settings_clock(clamped);
                    ui.set_settings_fg(fg);
                    ui.set_settings_bg(bg);
                    ui.set_show_icons(si);
                    ui.set_show_all(sa);
                }
            }
        });

        ui.on_game_selected({
            let cfg = config.clone();
            let state = emulator_state.clone();
            let ui_weak = ui.as_weak();

            move |title| {
                if let Some(entry) = cfg.borrow().roms.iter().find(|r| *r.title == *title)
                    && let Ok(bytes) = std::fs::read(&entry.path)
                {
                    let mut st = state.borrow_mut();
                    st.active_cpu.clear();

                    if st.active_cpu.load_rom(&bytes).is_ok() {
                        st.is_running = true;
                        st.hovered_index = None;

                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_in_game(true);
                        }
                    }
                }
            }
        });

        ui.on_game_hovered({
            let state = emulator_state.clone();
            let cfg = config.clone();

            move |idx| {
                let mut st = state.borrow_mut();

                if idx < 0 {
                    st.hovered_index = None;
                    return;
                }

                let i = idx as usize;
                st.hovered_index = Some(i);

                // Lazy initialization
                if let Some(slot) = st.previews.get_mut(i)
                    && slot.is_none()
                    && let Some(rom) = cfg.borrow().roms.get(i)
                {
                    *slot = PreviewInstance::from_path(&rom.path);
                }
            }
        });

        ui.on_back_to_launcher({
            let state = emulator_state.clone();
            let ui_weak = ui.as_weak();

            move || {
                let mut st = state.borrow_mut();

                st.active_cpu.clear();
                st.is_running = false;
                st.sink.pause();

                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_in_game(false);
                }
            }
        });

        ui.on_press_key({
            let state = emulator_state.clone();

            move |_key| {
                if let Some(key) = map_key(_key.as_str()) {
                    state.borrow_mut().active_cpu.keypad[key] = true;
                }
            }
        });

        ui.on_release_key({
            let state = emulator_state.clone();

            move |_key| {
                if let Some(key) = map_key(_key.as_str()) {
                    state.borrow_mut().active_cpu.keypad[key] = false;
                }
            }
        });

        let ui_weak = ui.as_weak();
        let shared_for_timer = shared_settings.clone();
        let state_for_timer = emulator_state.clone();
        let cfg_for_timer = config.clone();
        let model_for_timer = games_model.clone();
        let timer = slint::Timer::default();

        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(16),
            move || {
                let s = shared_for_timer.borrow().clone();
                let ticks = (s.clock_frequency / 60).max(1);
                let mut st = state_for_timer.borrow_mut();

                let Some(ui) = ui_weak.upgrade() else {
                    return;
                };

                if ui.get_show_settings() {
                    if !st.sink.is_paused() {
                        st.sink.pause();
                    }
                    return;
                }

                if st.is_running {
                    for _ in 0..ticks {
                        if st.active_cpu.tick().is_err() {
                            st.is_running = false;
                            st.sink.pause();
                            return;
                        }
                    }
                    st.active_cpu.update_timers();

                    if st.active_cpu.sound_timer > 0 {
                        if st.sink.is_paused() {
                            st.sink.play();
                        }
                    } else if !st.sink.is_paused() {
                        st.sink.pause();
                    }

                    let image =
                        convert_framebuffer(&st.active_cpu.display, s.fg_color, s.bg_color, 4);

                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_screen(image);
                    }
                } else if !ui.get_in_game() && s.show_icons {
                    let indices: Vec<usize> = if s.show_all {
                        (0..st.previews.len()).collect()
                    } else {
                        st.hovered_index.into_iter().collect()
                    };

                    for i in indices {
                        if let Some(slot) = st.previews.get_mut(i) {
                            if slot.is_none()
                                && let Some(rom) = cfg_for_timer.borrow().roms.get(i)
                            {
                                *slot = PreviewInstance::from_path(&rom.path);
                            }

                            if let Some(preview) = slot {
                                if !preview.halted {
                                    for _ in 0..ticks {
                                        if preview.cpu.tick().is_err() {
                                            preview.halted = true;
                                            break;
                                        }
                                    }
                                    preview.cpu.update_timers();
                                }

                                let img = convert_framebuffer(
                                    &preview.cpu.display,
                                    s.fg_color,
                                    s.bg_color,
                                    2,
                                );

                                if let Some(mut row) = model_for_timer.row_data(i) {
                                    row.icon = img;
                                    model_for_timer.set_row_data(i, row);
                                }
                            }
                        }
                    }
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

    #[test]
    fn test_legacy_config_migration() {
        let legacy_json = r#"{
            "roms": [],
            "settings": {
                "clock_frequency": 500,
                "fg_color": [255, 255, 255],
                "bg_color": [0, 0, 0]
            }
        }"#;

        let config: LibraryConfig = serde_json::from_str(legacy_json).unwrap();

        assert_eq!(config.settings.clock_frequency, 500);
        assert_eq!(config.settings.fg_color, [255, 255, 255]);
        assert_eq!(config.settings.bg_color, [0, 0, 0]);

        assert!(config.settings.show_icons);
        assert!(!config.settings.show_all);
    }

    #[test]
    fn test_preview_from_path() {
        assert!(PreviewInstance::from_path(Path::new("no_such_rom_12345.ch8")).is_none());

        let temp_dir = std::env::temp_dir();

        let ok_path = temp_dir.join("test_preview_ok.ch8");
        std::fs::write(&ok_path, [0x00, 0xE0]).unwrap();
        let ok_preview = PreviewInstance::from_path(&ok_path).unwrap();

        assert!(!ok_preview.halted);

        let _ = std::fs::remove_file(&ok_path);

        let big_path = temp_dir.join("test_preview_big.ch8");
        std::fs::write(&big_path, vec![0u8; 3585]).unwrap();
        let big_preview = PreviewInstance::from_path(&big_path).unwrap();

        assert!(big_preview.halted);

        let _ = std::fs::remove_file(&big_path);
    }

    #[test]
    fn test_map_key() {
        assert_eq!(map_key("q"), Some(0x4));
        assert_eq!(map_key("Q"), Some(0x4));
        assert_eq!(map_key("й"), Some(0x4));
        assert_eq!(map_key("Й"), Some(0x4));
        assert_eq!(map_key("x"), Some(0x0));
        assert_eq!(map_key("ч"), Some(0x0));
        assert_eq!(map_key(" "), None);
    }
}
