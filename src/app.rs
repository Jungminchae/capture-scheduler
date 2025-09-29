use crate::config::{Config, Rect, ScheduleTime};
use crate::scheduler;
use eframe::{App, Frame, egui};
use egui::{Color32, Id, Key, Pos2, TextureHandle, UserAttentionType, ViewportCommand};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum AppMessage {
    OpenSettings,
    Exit,
}

enum AppState {
    Hidden,
    Settings,
    WaitingForCapture(Arc<Mutex<Option<PathBuf>>>),
    Selecting,
}

pub struct CaptureSchedulerApp {
    config: Config,
    new_schedule_time: String,
    state: AppState,
    selection_start_pos: Option<Pos2>,
    selection_background: Option<TextureHandle>,
    last_scheduler_check: Instant,
    app_rx: mpsc::Receiver<AppMessage>,
    _tray: TrayIcon,
}

impl CaptureSchedulerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (app_tx, app_rx) = mpsc::channel();

        let tray = {
            let icon_data = include_bytes!("../assets/icon.jpg").to_vec();
            let icon = image::load_from_memory(&icon_data)
                .expect("Failed to load icon. Make sure '/assets/icon.jpg' exists.")
                .to_rgba8();
            let (width, height) = icon.dimensions();
            let tray_icon = tray_icon::Icon::from_rgba(icon.into_raw(), width, height)
                .expect("Failed to create tray icon.");

            let menu = Menu::new();
            let settings_item = MenuItem::new("Settings", true, None);
            let quit_item = MenuItem::new("Exit", true, None);
            menu.append_items(&[&settings_item, &quit_item]).unwrap();

            let settings_id = settings_item.id().clone();
            let quit_id = quit_item.id().clone();
            let s_clone = app_tx.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                if event.id == settings_id {
                    s_clone.send(AppMessage::OpenSettings).unwrap();
                } else if event.id == quit_id {
                    s_clone.send(AppMessage::Exit).unwrap();
                }
            }));

            TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_icon(tray_icon)
                .build()
                .unwrap()
        };

        cc.egui_ctx
            .send_viewport_cmd(ViewportCommand::Visible(false));

        Self {
            config: Config::load().unwrap_or_default(),
            new_schedule_time: "HH:MM:SS".to_string(),
            state: AppState::Hidden,
            selection_start_pos: None,
            selection_background: None,
            last_scheduler_check: Instant::now(),
            app_rx,
            _tray: tray,
        }
    }
}

impl App for CaptureSchedulerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        if let Ok(msg) = self.app_rx.try_recv() {
            match msg {
                AppMessage::OpenSettings => {
                    self.state = AppState::Settings;
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::RequestUserAttention(
                        UserAttentionType::Informational,
                    ));
                }
                AppMessage::Exit => {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }

        let mut next_state = None;

        match &mut self.state {
            AppState::Hidden => {
                if self.last_scheduler_check.elapsed() > Duration::from_secs(1) {
                    scheduler::check_and_capture(&self.config);
                    self.last_scheduler_check = Instant::now();
                }
                ctx.request_repaint_after(Duration::from_secs(1));
            }
            AppState::Settings => {
                next_state = self.settings_ui(ctx);
            }
            AppState::WaitingForCapture(capture_result) => {
                let mut result_guard = capture_result.lock().unwrap();
                if let Some(path) = result_guard.take() {
                    if let Ok(image_data) = std::fs::read(path) {
                        if let Ok(image) = image::load_from_memory(&image_data) {
                            let size = [image.width() as _, image.height() as _];
                            let image_buffer = image.to_rgba8();
                            let egui_image =
                                egui::ColorImage::from_rgba_unmultiplied(size, &image_buffer);
                            self.selection_background = Some(ctx.load_texture(
                                "screenshot",
                                egui_image,
                                Default::default(),
                            ));

                            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
                            ctx.send_viewport_cmd(ViewportCommand::Decorations(false));
                            next_state = Some(AppState::Selecting);
                        } else {
                            next_state = Some(AppState::Settings);
                        }
                    } else {
                        next_state = Some(AppState::Settings);
                    }
                } else {
                    ctx.request_repaint();
                }
            }
            AppState::Selecting => {
                next_state = self.selection_ui(ctx);
            }
        }

        if let Some(state) = next_state {
            self.state = state;
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        match self.state {
            AppState::Settings => [0.1, 0.1, 0.1, 1.0],
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    }
}

impl CaptureSchedulerApp {
    fn settings_ui(&mut self, ctx: &egui::Context) -> Option<AppState> {
        let mut next_state = None;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Capture Scheduler Settings");
            ui.add_space(10.0);

            ui.label("Capture Area");
            ui.horizontal(|ui| {
                if ui.button("Select capture area").clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                    let result = Arc::new(Mutex::new(None));
                    next_state = Some(AppState::WaitingForCapture(Arc::clone(&result)));

                    thread::spawn(move || {
                        let exe_path = std::env::current_exe()
                            .unwrap()
                            .parent()
                            .unwrap()
                            .join("capture-cli");
                        if let Ok(output) = Command::new(exe_path).output() {
                            if output.status.success() {
                                let path_str = String::from_utf8_lossy(&output.stdout);
                                *result.lock().unwrap() = Some(PathBuf::from(path_str.trim()));
                            }
                        }
                    });
                }
                if let Some(rect) = self.config.capture_area {
                    ui.label(format!(
                        "{}x{} at ({}, {})",
                        rect.width, rect.height, rect.x, rect.y
                    ));
                } else {
                    ui.label("Not selected");
                }
            });

            ui.separator();

            ui.label("Schedule Times (HH:MM:SS)");
            let mut to_remove = None;
            for (i, schedule) in self.config.schedule_times.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{:02}:{:02}:{:02}",
                        schedule.hour, schedule.minute, schedule.second
                    ));
                    if ui.button("x").clicked() {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(i) = to_remove {
                self.config.schedule_times.remove(i);
            }

            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_schedule_time);
                if ui.button("Add").clicked() {
                    let parts: Vec<Result<u32, _>> = self
                        .new_schedule_time
                        .split(':')
                        .map(|s| s.parse())
                        .collect();
                    if parts.len() == 2 && parts.iter().all(|p| p.is_ok()) {
                        let nums: Vec<u32> = parts.into_iter().map(|p| p.unwrap()).collect();
                        self.config.schedule_times.push(ScheduleTime {
                            hour: nums[0],
                            minute: nums[1],
                            second: nums[2],
                        });
                        self.new_schedule_time = "HH:MM:SS".to_string();
                    }
                }
            });

            ui.separator();

            ui.label("Save Directory");
            ui.horizontal(|ui| {
                if ui.button("Change folder").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.config.save_directory = path;
                    }
                }
                ui.label(
                    self.config
                        .save_directory
                        .to_str()
                        .unwrap_or("Invalid path"),
                );
            });

            ui.separator();

            if ui.button("Save and Hide").clicked() {
                if let Err(e) = self.config.save() {
                    eprintln!("Failed to save config: {}", e);
                }
                ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                next_state = Some(AppState::Hidden);
            }
        });
        next_state
    }

    fn selection_ui(&mut self, ctx: &egui::Context) -> Option<AppState> {
        ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        let mut should_exit_selection = false;
        let mut selected_rect_logical = None;

        if let Some(texture) = &self.selection_background {
            egui::Area::new(Id::new("selection_area"))
                .fixed_pos(Pos2::ZERO)
                .show(ctx, |ui| {
                    let screen_rect = ctx.screen_rect();
                    ui.image((texture.id(), screen_rect.size()));

                    ui.painter()
                        .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(120));
                    let input = ui.input(|i| i.clone());

                    if input.key_pressed(Key::Escape) {
                        should_exit_selection = true;
                    }
                    if input.pointer.primary_down() {
                        if self.selection_start_pos.is_none() {
                            self.selection_start_pos = input.pointer.interact_pos();
                        }
                    }
                    if let (Some(start_pos), Some(current_pos)) =
                        (self.selection_start_pos, input.pointer.interact_pos())
                    {
                        let selection_rect = egui::Rect::from_two_pos(start_pos, current_pos);
                        ui.painter().rect_stroke(
                            selection_rect,
                            0.0,
                            egui::Stroke::new(2.0, Color32::WHITE),
                        );
                        if input.pointer.primary_released() {
                            selected_rect_logical = Some(selection_rect);
                            should_exit_selection = true;
                        }
                    }
                });
        } else {
            should_exit_selection = true;
        }

        if should_exit_selection {
            match selected_rect_logical {
                Some(rect) => {
                    self.config.capture_area = Some(Rect {
                        x: rect.min.x.round() as i32,
                        y: rect.min.y.round() as i32,
                        width: rect.width().round() as u32,
                        height: rect.height().round() as u32,
                        scale_factor: ctx.pixels_per_point(),
                    });
                }
                _ => (),
            }
            self.selection_background = None;
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
            ctx.send_viewport_cmd(ViewportCommand::Decorations(true));
            ctx.set_cursor_icon(egui::CursorIcon::Default);
            return Some(AppState::Settings);
        }
        None
    }
}
