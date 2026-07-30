#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
mod particle_mesh;
use crate::particle_mesh::ParticleMesh;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SetWindowLongW, GetWindowLongW, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
};
use crate::tray::TrayHandler;
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIcon;

mod ram_monitor;
mod tray;

// Embed icon bytes at compile-time
static ICON_BYTES: &[u8] = include_bytes!("../icon.png");

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ShortcutConfig {
    name: String,
    exe_path: String,
    icon_png_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct WindowConfig {
    id_token: u32,
    title: String,
    pos_x: f32,
    pos_y: f32,
    width: f32,
    height: f32,
    shortcuts: Vec<ShortcutConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct AppConfig {
    windows: Vec<WindowConfig>,
    next_id: u32,
}

impl AppConfig {
    fn load() -> Self {
        let path = Path::new("dashboard_config.json");
        if !path.exists() {
            return AppConfig::default();
        }
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return AppConfig::default(),
        };
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            serde_json::from_str(&contents).unwrap_or_else(|_| AppConfig::default())
        } else {
            AppConfig::default()
        }
    }

    fn save(&self) {
        if let Ok(json_string) = serde_json::to_string_pretty(self) {
            if let Ok(mut file) = File::create("dashboard_config.json") {
                let _ = file.write_all(json_string.as_bytes());
            }
        }
    }
}

fn main() -> eframe::Result {
    let window_title = "AppOverlayCanvas";
    std::fs::create_dir_all("_icon_cache").ok();

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_title(window_title)
            .with_inner_size([320.0, 480.0])
            .with_transparent(true)
            .with_decorations(false)
            .with_resizable(true)
            .with_taskbar(false),
        ..Default::default()
    };

    eframe::run_native(
        window_title,
        options,
        Box::new(move |_cc| {
            apply_win32_layers(window_title);

            let ram_state = Arc::new(Mutex::new(ram_monitor::RamUsage::read()));
            let ram_for_thread = ram_state.clone();

            // Background thread updates RAM stats every 3 seconds
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(3));
                let new_ram = ram_monitor::RamUsage::read();
                if let Ok(mut ram) = ram_for_thread.lock() {
                    *ram = new_ram;
                }
            });

            let tray_handler = TrayHandler::new(ICON_BYTES);
            let saved_config = AppConfig::load();

            let app = DeskFrameApp {
                _tray_icon: tray_handler.icon,
                show_id: tray_handler.show_id.clone(),
                exit_id: tray_handler.exit_id.clone(),
                text_color: egui::Color32::WHITE,
                next_window_id: saved_config.next_id,
                persistent_config: Arc::new(Mutex::new(saved_config)),
                ram_usage: ram_state,
                particle_mesh: ParticleMesh::new(50, (320.0, 480.0)),
                exe_input_buffer: String::new(),
                name_input_buffer: String::new(),
            };

            Ok(Box::new(app))
        }),
    )
}

struct DeskFrameApp {
    _tray_icon: TrayIcon,
    show_id: tray_icon::menu::MenuId,
    exit_id: tray_icon::menu::MenuId,
    text_color: egui::Color32,
    next_window_id: u32,
    persistent_config: Arc<Mutex<AppConfig>>,
    ram_usage: Arc<Mutex<ram_monitor::RamUsage>>,
    exe_input_buffer: String,
    name_input_buffer: String,
    particle_mesh: ParticleMesh,
}

impl eframe::App for DeskFrameApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.exit_id {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else if event.id == self.show_id {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        let custom_frame = egui::Frame::NONE
            .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 20, 150))
            .corner_radius(12.0)
            .inner_margin(16.0);

        egui::CentralPanel::default()
            .frame(custom_frame)
            .show_inside(ui, |ui| {
                let dt = ui.ctx().input(|x| x.unstable_dt);
                let size = ui.ctx().content_rect().size();

                self.particle_mesh.resize((size.x, size.y));
                self.particle_mesh.update(dt, (size.x, size.y));
                self.particle_mesh.draw(ui.painter());

                // Keep particles moving smoothly continuously
                ui.ctx().request_repaint();

                let bg_drag = ui.interact(
                    ui.max_rect(),
                    egui::Id::new("widget_drag_layer"),
                    egui::Sense::drag(),
                );
                if bg_drag.dragged() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                ui.heading("RustyFrame Dashboard");
                ui.separator();

                // Display cached RAM usage safely
                let ram_text = if let Ok(ram) = self.ram_usage.lock() {
                    ram_monitor::RamUsage::format_bytes(ram.used_kb)
                } else {
                    "N/A".to_string()
                };
                let ram_status = format!("RAM USAGE: {}", ram_text);
                ui.label(egui::RichText::new(&ram_status).color(self.text_color));

                ui.add_space(4.0);

                ui.label(
                    egui::RichText::new("Shortcut Canvas Controller").color(self.text_color),
                );
                ui.add_space(8.0);

                ui.label("New Window Shortcut Destination (.exe path):");
                ui.text_edit_singleline(&mut self.exe_input_buffer);
                ui.label("Display Name Target:");
                ui.text_edit_singleline(&mut self.name_input_buffer);
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui
                        .button("➕ Create Shortcut Canvas Window")
                        .clicked()
                        && !self.exe_input_buffer.is_empty()
                    {
                        let target_exe = self.exe_input_buffer.trim().to_string();
                        let target_name = if self.name_input_buffer.is_empty() {
                            "Shortcut Layer".to_string()
                        } else {
                            self.name_input_buffer.trim().to_string()
                        };
                        let cache_png_name =
                            format!("_icon_cache/icon_{}.png", self.next_window_id);
                        if let Ok(ref png_bytes) =
                            win_icon_extractor::extract_icon_png(&target_exe)
                        {
                            if let Ok(mut f) = File::create(&cache_png_name) {
                                let _ = f.write_all(png_bytes);
                            }
                        }
                        let new_window = WindowConfig {
                            id_token: self.next_window_id,
                            title: target_name.clone(),
                            pos_x: 150.0,
                            pos_y: 150.0,
                            width: 280.0,
                            height: 220.0,
                            shortcuts: vec![ShortcutConfig {
                                name: target_name,
                                exe_path: target_exe,
                                icon_png_path: cache_png_name,
                            }],
                        };
                        if let Ok(mut cfg) = self.persistent_config.lock() {
                            cfg.windows.push(new_window);
                            self.next_window_id += 1;
                            cfg.next_id = self.next_window_id;
                            cfg.save();
                        }
                        self.exe_input_buffer.clear();
                        self.name_input_buffer.clear();
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button("❌ Close App").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("📥 Hide to Tray").clicked() {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                });

                let active = self.persistent_config.lock().expect("lock");
                for w in &active.windows {
                    let child_id =
                        egui::ViewportId::from_hash_of(format!("canvas_layer_{}", w.id_token));
                    let db_handle = self.persistent_config.clone();
                    let title_value = w.title.clone();
                    let shortcuts_value = w.shortcuts.clone();
                    let id = w.id_token;
                    let width = w.width;
                    let height = w.height;

                    ui.ctx().show_viewport_deferred(
                        child_id,
                        egui::ViewportBuilder::default()
                            .with_title(&title_value)
                            .with_inner_size([width, height])
                            .with_transparent(true)
                            .with_decorations(false)
                            .with_taskbar(false),
                        move |ctx, _| {
                            let frame = egui::Frame::NONE
                                .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                                .corner_radius(10.0)
                                .inner_margin(14.0);
                            let db_clone = db_handle.clone();
                            egui::CentralPanel::default().frame(frame).show_inside(ctx, |ui| {
                                let drag = ui.interact(
                                    ui.max_rect(),
                                    egui::Id::new(format!("drag_{}", id)),
                                    egui::Sense::drag(),
                                );
                                if drag.dragged() {
                                    ui.ctx().send_viewport_cmd(
                                        egui::ViewportCommand::StartDrag,
                                    );
                                }
                                ui.heading(&title_value);
                                ui.separator();
                                for sc in &shortcuts_value {
                                    ui.vertical_centered(|ui| {
                                        if ui.button(format!("🚀 Launch {}", sc.name)).clicked() {
                                            let _ = std::process::Command::new(&sc.exe_path)
                                                .spawn();
                                        }
                                        ui.small(&sc.exe_path);
                                    });
                                }
                                ui.with_layout(
                                    egui::Layout::bottom_up(egui::Align::LEFT),
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            if ui.button("🗑️ Remove Widget").clicked() {
                                                if let Ok(mut cfg) = db_clone.lock() {
                                                    cfg.windows.retain(|w| w.id_token != id);
                                                    cfg.save();
                                                }
                                                ui.ctx().send_viewport_cmd(
                                                    egui::ViewportCommand::Close,
                                                );
                                            }
                                            if ui.button("💾 Save Bounds").clicked() {
                                                if let Ok(mut cfg) = db_clone.lock() {
                                                    if let Some(tgt) = cfg
                                                        .windows
                                                        .iter_mut()
                                                        .find(|w| w.id_token == id)
                                                    {
                                                        tgt.width = ui.ctx().content_rect().width();
                                                        tgt.height = ui.ctx().content_rect().height();
                                                        cfg.save();
                                                    }
                                                }
                                            }
                                        });
                                    },
                                );
                            });
                        },
                    );
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    let res = ui.allocate_response(egui::vec2(16.0, 16.0), egui::Sense::drag());
                    ui.painter().text(
                        res.rect.right_bottom(),
                        egui::Align2::RIGHT_BOTTOM,
                        "📐",
                        egui::FontId::proportional(12.0),
                        egui::Color32::from_gray(255),
                    );
                    if res.dragged() {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::BeginResize(
                                egui::ResizeDirection::SouthEast,
                            ));
                    }
                });
            });
    }
}

fn apply_win32_layers(title: &str) {
    unsafe {
        let mut wide: Vec<u16> = title.encode_utf16().collect();
        wide.push(0);
        if let Some(hwnd) = FindWindowW(None, windows::core::PCWSTR(wide.as_ptr())).ok() {
            let style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            // Hide from alt-tab taskbar without forcing focus/topmost Z-order
            let new_style = style | WS_EX_TOOLWINDOW.0 as i32;
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
        }
    }
}