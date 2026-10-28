use slint::{ModelRc, SharedString, VecModel};
use std::rc::Rc;

slint::include_modules!();

pub struct App {
    ui: MainWindow,
    //emu: Rc<RefCell<core::Emulaor>>,
    _games_model: Rc<VecModel<GameEntry>>,
}
impl App {
    pub fn new() -> Result<Self, slint::PlatformError> {
        let ui = MainWindow::new()?;

        let games = vec![
            GameEntry {
                title: SharedString::from("Pong"),
                size: SharedString::from("284 B"),
                icon: slint::Image::default(),
            },
            GameEntry {
                title: SharedString::from("Tetris"),
                size: SharedString::from("1.2 KB"),
                icon: slint::Image::default(),
            },
            GameEntry {
                title: SharedString::from("Space Invaders"),
                size: SharedString::from("2.1 KB"),
                icon: slint::Image::default(),
            },
        ];

        let games_model = Rc::new(VecModel::from(games));
        ui.set_games(ModelRc::from(games_model.clone()));

        ui.on_game_selected({
            let ui_weak = ui.as_weak();
            move |_title| {
                let ui = ui_weak.unwrap();
                ui.set_in_game(true);
            }
        });

        ui.on_back_to_launcher({
            let ui_weak = ui.as_weak();
            move || {
                ui_weak.unwrap().set_in_game(false);
            }
        });

        ui.on_settings_clicked({
            move || {
                println!("Settings clicked");
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
