use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SetWindowPos, HWND_BOTTOM, SWP_NOMOVE, SWP_NOSIZE, 
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

fn main() -> eframe::Result {
    let window_title = "AppOverlayCanvas";

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
        Box::new(move |cc| {
            apply_win32_layers(window_title);

            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_millis(500));
                    apply_win32_layers(window_title);
                    ctx.request_repaint(); 
                }
            });

            // --- SYSTEM TRAY INITIALIZATION ---
            let icon_path = std::path::Path::new("icon.png");
            let icon = load_icon(icon_path);

            let tray_menu = Menu::new();
            let show_item = MenuItem::new("Show Window", true, None);
            let exit_item = MenuItem::new("Exit", true, None);
            let _ = tray_menu.append_items(&[&show_item, &exit_item]);

            let tray_icon = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("Application")
                .with_icon(icon)
                .build()
                .unwrap();

            let app = DeskFrameApp {
                _tray_icon: tray_icon,
                show_id: show_item.id().clone(),
                exit_id: exit_item.id().clone(),
                text_color: egui::Color32::from_rgb(0, 255, 150),
                
                // Track all open window IDs inside a vector array list
                open_windows: Arc::new(Mutex::new(Vec::new())),
                next_window_id: 0,
            };

            Ok(Box::new(app))
        }),
    )
}

fn load_icon(path: &std::path::Path) -> Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon.png. Place it directly next to your Cargo.toml")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to create tray icon")
}

fn apply_win32_layers(title: &str) {
    unsafe {
        let mut title_wide: Vec<u16> = title.encode_utf16().collect();
        title_wide.push(0);
        if let Some(hwnd) = FindWindowW(None, windows::core::PCWSTR(title_wide.as_ptr())).ok() {
            use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongW, SetWindowLongW, GWL_EXSTYLE};
            let current_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            let new_style = current_style | WS_EX_NOACTIVATE.0 as i32 | WS_EX_TOOLWINDOW.0 as i32;
            let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, new_style);
            let _ = SetWindowPos(hwnd, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        }
    }
}

// 3. The State Struct 
struct DeskFrameApp {
    _tray_icon: TrayIcon,
    show_id: tray_icon::menu::MenuId,
    exit_id: tray_icon::menu::MenuId,
    text_color: egui::Color32,
    
    // Memory handles to track dynamic multiple window instances
    open_windows: Arc<Mutex<Vec<u32>>>,
    next_window_id: u32,
}

// 4. The eframe Application implementation
impl eframe::App for DeskFrameApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.exit_id {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else if event.id == self.show_id {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(true));
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
                let bg_drag = ui.interact(ui.max_rect(), egui::Id::new("widget_drag_layer"), egui::Sense::drag());
                if bg_drag.dragged() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                ui.heading("Welcome to Rustframe!");
                ui.separator();
                
                ui.label(egui::RichText::new("Entirely written in rust!").color(self.text_color));
                
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("❌ Close").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("📥 Hide").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    
                    // Clicking adds a brand new unique ID index token into our open list tracker
                    if ui.button("➕ New Window").clicked() {
                        if let Ok(mut windows) = self.open_windows.lock() {
                            windows.push(self.next_window_id);
                            self.next_window_id += 1;
                        }
                    }
                });

                // Grab a clean snapshot copy of active windows to iterate over safely
                let active_window_ids = if let Ok(windows) = self.open_windows.lock() {
                    windows.clone()
                } else {
                    Vec::new()
                };

                // Loop through and build each window completely independently using its unique token index ID
                for id_token in active_window_ids {
                    // Create a completely distinct viewport identifier string key for egui engine context routing
                    let child_id = egui::ViewportId::from_hash_of(format!("secondary_canvas_window_{}", id_token));
                    let list_pointer_handle = self.open_windows.clone();

                    ui.ctx().show_viewport_deferred(
                        child_id,
                        egui::ViewportBuilder::default()
                            .with_title(format!("Sub Window Layer {}", id_token))
                            .with_inner_size([250.0, 200.0])
                            .with_transparent(true)
                            .with_decorations(false)
                            .with_taskbar(false), // Hidden from the taskbar natively
                        move |ctx, ui| {
                            let child_frame = egui::Frame::NONE
                                .fill(egui::Color32::from_rgba_unmultiplied(35, 35, 35, 200))
                                .corner_radius(8.0)
                                .inner_margin(12.0);

                            let state_clone = list_pointer_handle.clone();

                            egui::CentralPanel::default()
                                .frame(child_frame)
                                .show(ctx, move |ui| {
                                    let child_drag = ui.interact(ui.max_rect(), egui::Id::new(format!("child_drag_{}", id_token)), egui::Sense::drag());
                                    if child_drag.dragged() {
                                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                                    }

                                    ui.heading(format!("Window #{}", id_token));
                                    ui.separator();
                                    ui.label("This is an isolated, dynamic desktop canvas layer.");
                                    ui.add_space(10.0);
                                    
                                    // Dismiss removes this specific item token from the tracking array stack
                                    if ui.button("Dismiss").clicked() {
                                        if let Ok(mut windows) = state_clone.lock() {
                                            windows.retain(|&w_id| w_id != id_token);
                                        }
                                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                    }
                                });
                        },
                    );
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    let resize_response = ui.allocate_response(egui::vec2(16.0, 16.0), egui::Sense::drag());
                    
                    ui.painter().text(
                        resize_response.rect.right_bottom(),
                        egui::Align2::RIGHT_BOTTOM,
                        "📐",
                        egui::FontId::proportional(12.0),
                        egui::Color32::from_gray(255), 
                    );

                    if resize_response.dragged() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::BeginResize(egui::ResizeDirection::SouthEast));
                    }
                });
            });
    }
}