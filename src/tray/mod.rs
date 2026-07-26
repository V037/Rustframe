use tray_icon::{menu::{Menu, MenuItem}, Icon, TrayIcon, TrayIconBuilder};
use image;

pub struct TrayHandler {
    pub icon: TrayIcon,
    pub show_id: tray_icon::menu::MenuId,
    pub exit_id: tray_icon::menu::MenuId,
}

impl TrayHandler {
    pub fn new(icon_path: &std::path::Path) -> Self {
        let icon = load_icon(icon_path);
        let mut menu = Menu::new();
        let show_item = MenuItem::new("Show Window", true, None);
        let exit_item = MenuItem::new("Exit", true, None);
        menu.append_items(&[&show_item, &exit_item]);

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Application")
            .with_icon(icon)
            .build()
            .unwrap();

        Self {
            icon: tray_icon,
            show_id: show_item.id().clone(),
            exit_id: exit_item.id().clone(),
        }
    }
}

fn load_icon(path: &std::path::Path) -> Icon {
    let (rgba, w, h) = {
        let img = image::open(path).expect("Failed to open icon.png").into_rgba8();
        let (w, h) = img.dimensions();
        (img.into_raw(), w, h)
    };
    Icon::from_rgba(rgba, w, h).expect("Failed to create tray icon")
}
