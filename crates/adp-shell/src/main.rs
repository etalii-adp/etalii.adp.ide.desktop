//! The desktop host's window (etalii.adp spec 030, research R2, task T021): one native window per workspace, drawn
//! with egui on wgpu, exposed to screen readers through AccessKit, in VS Code's default theme following the system.
//! The window itself is the `adp_shell` library; this is the process that runs it.

#![cfg_attr(not(test), windows_subsystem = "windows")]

use adp_shell::{shell, theme};
use eframe::egui;

struct App {
    shell: shell::Shell,
    /// The start-up probe (spec 030, T090, research R12): the file `ADP_STARTUP_PROBE` names, and the frames drawn.
    probe: Option<(std::path::PathBuf, usize)>,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.show(ui);
        if let Some((file, frames)) = &mut self.probe {
            *frames += 1;
            if *frames == 1 {
                // The first frame is on the screen once the second one starts.
                ui.ctx().request_repaint();
            } else {
                // When it was, as the time since 1970 in nanoseconds, which the probe's caller compares with when it
                // started the process; then the window closes.
                let shown = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or_default();
                let _ = std::fs::write(file, shown.to_string());
                self.probe = None;
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
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
            // `ADP_STATE` keeps the state in a folder of the caller's choosing, as a portable install or the start-up
            // probe does; else the platform's folder.
            let state = match std::env::var_os("ADP_STATE") {
                Some(folder) => Ok(adp_core::state::StateFolder::at(folder)),
                None => adp_core::state::StateFolder::for_platform(),
            };
            let mut shell = match state {
                Ok(state) => shell::Shell::with_state(state),
                Err(_) => shell::Shell::default(),
            };
            // `adp-desktop <folder>` opens a project and `adp-desktop <file>` a file, as `code <path>` does.
            if let Some(path) = std::env::args_os().nth(1) {
                shell.open_path(std::path::Path::new(&path));
            }
            let probe = std::env::var_os("ADP_STARTUP_PROBE").map(|file| (file.into(), 0));
            Ok(Box::new(App { shell, probe }))
        }),
    )
}
