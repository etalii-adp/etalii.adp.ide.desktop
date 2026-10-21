//! The desktop host's window (etalii.adp spec 030, research R2, task T021): one native window per workspace, drawn
//! with egui on wgpu, exposed to screen readers through AccessKit, in VS Code's default theme following the system.

#![cfg_attr(not(test), windows_subsystem = "windows")]

mod editor;
mod highlight;
mod markdown;
mod shell;
mod theme;

use eframe::egui;

struct App {
    shell: shell::Shell,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.show(ui);
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ADP")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ADP",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            let mut shell = shell::Shell::default();
            // `adp-desktop <file>` opens the file in the editor, as `code <file>` does.
            if let Some(path) = std::env::args_os().nth(1) {
                shell.open_text(std::path::Path::new(&path));
            }
            Ok(Box::new(App { shell }))
        }),
    )
}
