use eframe::egui;
use eframe::run_native;

use crate::app::launcher_app::LAUNCHER_APP_SIZE;
use crate::app::launcher_app::LauncherApp;
use crate::config::build_config;
use crate::config::runtime_config::Config;
use crate::update_app::app::UPDATE_APP_SIZE;
use crate::update_app::app::{UpdateApp, should_check_updates};
use crate::utils;

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    Updating,
    Launcher,
}

pub struct UnifiedApp {
    app_state: AppState,

    update_app: Option<UpdateApp>,
    launcher_app: Option<LauncherApp>,

    config: Option<Config>,
    launch_flag: bool,
}

pub fn run_gui(config: Config, launch: bool) {
    let should_check_updates = should_check_updates();
    let initial_size = if should_check_updates {
        UPDATE_APP_SIZE
    } else {
        LAUNCHER_APP_SIZE
    };
    let app_name = if should_check_updates {
        format!("{} Updater", build_config::get_launcher_name())
    } else {
        build_config::get_launcher_name()
    };

    let native_options = eframe::NativeOptions {
        viewport: utils::add_icon(
            egui::ViewportBuilder::default()
                .with_inner_size(initial_size)
                .with_resizable(true),
        ),
        centered: true,
        ..Default::default()
    };

    run_native(
        &app_name,
        native_options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            crate::app::theme::install(&cc.egui_ctx);
            Ok(Box::new(UnifiedApp::new(
                config,
                &cc.egui_ctx,
                launch,
                should_check_updates,
            )))
        }),
    )
    .unwrap();
}

impl eframe::App for UnifiedApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        match self.app_state {
            AppState::Updating => {
                if let Some(update_app) = &mut self.update_app {
                    update_app.update(ctx, frame);

                    if update_app.should_proceed_to_launcher() {
                        self.transition_to_launcher(ctx);
                    }
                }
            }
            AppState::Launcher => {
                if let Some(launcher_app) = &mut self.launcher_app {
                    launcher_app.update(ctx, frame);
                }
            }
        }
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        match self.app_state {
            AppState::Updating => {
                if let Some(update_app) = &self.update_app {
                    update_app.clear_color(visuals)
                } else {
                    visuals.window_fill.to_normalized_gamma_f32()
                }
            }
            AppState::Launcher => {
                if let Some(launcher_app) = &self.launcher_app {
                    launcher_app.clear_color(visuals)
                } else {
                    visuals.window_fill.to_normalized_gamma_f32()
                }
            }
        }
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        match self.app_state {
            AppState::Updating => {
                if let Some(update_app) = &mut self.update_app {
                    update_app.on_exit(gl);
                }
            }
            AppState::Launcher => {
                if let Some(launcher_app) = &mut self.launcher_app {
                    launcher_app.on_exit(gl);
                }
            }
        }
    }
}

impl UnifiedApp {
    fn new(config: Config, ctx: &egui::Context, launch: bool, should_check_updates: bool) -> Self {
        let app_state = if should_check_updates {
            AppState::Updating
        } else {
            AppState::Launcher
        };

        let mut app = UnifiedApp {
            app_state: app_state.clone(),
            update_app: None,
            launcher_app: None,
            config: Some(config),
            launch_flag: launch,
        };

        match app_state {
            AppState::Updating => {
                app.initialize_update_app(ctx);
            }
            AppState::Launcher => {
                app.initialize_launcher_app(ctx);
            }
        }

        app
    }

    fn initialize_update_app(&mut self, ctx: &egui::Context) {
        if let Some(config) = &self.config {
            self.update_app = Some(UpdateApp::new(config.lang, ctx));
        }
    }

    fn initialize_launcher_app(&mut self, ctx: &egui::Context) {
        if let Some(config) = self.config.take() {
            self.launcher_app = Some(LauncherApp::new(config, ctx, self.launch_flag));
        }
    }

    fn transition_to_launcher(&mut self, ctx: &egui::Context) {
        self.app_state = AppState::Launcher;
        self.update_app = None;

        let current_position = ctx
            .input(|input| input.viewport().outer_rect)
            .map(|rect| rect.min);

        if let Some(position) = current_position {
            let delta = LAUNCHER_APP_SIZE - UPDATE_APP_SIZE;
            let position = egui::Pos2::new(position.x - delta.x / 2.0, position.y - delta.y / 2.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(LAUNCHER_APP_SIZE));
        ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(false));

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(
            build_config::get_launcher_name(),
        ));

        self.initialize_launcher_app(ctx);
    }
}
