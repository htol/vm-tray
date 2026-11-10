//! Tray icon for libvirt VMs.
//!
//! The resident process only owns the tray icon (D-Bus). Clicking it spawns
//! `vm-tray popup`, a short-lived GTK process with the VM list.

mod options;
mod popup;
mod vm;

use std::process::{Child, Command};

use ksni::blocking::TrayMethods;

struct Tray {
    /// Options forwarded verbatim to the popup process.
    args: Vec<String>,
    popup: Option<Child>,
}

impl Tray {
    /// Closes the popup if it is open, otherwise opens it at the click position.
    fn toggle(&mut self, at: Option<(i32, i32)>) {
        if let Some(mut child) = self.popup.take()
            && matches!(child.try_wait(), Ok(None))
        {
            let _ = child.kill();
            let _ = child.wait();
            return;
        }
        let exe = std::env::current_exe().unwrap_or_else(|_| "vm-tray".into());
        let mut cmd = Command::new(exe);
        cmd.arg("popup").args(&self.args);
        if let Some((x, y)) = at {
            cmd.arg("--at").arg(format!("{x},{y}"));
        }
        match cmd.spawn() {
            Ok(child) => self.popup = Some(child),
            Err(e) => eprintln!("vm-tray: cannot start popup: {e}"),
        }
    }
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "vm-tray".into()
    }
    fn title(&self) -> String {
        "Virtual machines".into()
    }
    fn icon_name(&self) -> String {
        "vm-tray-symbolic".into()
    }
    /// `make install` puts the icon into `<PREFIX>/share/icons` next to
    /// `<PREFIX>/bin`. Pointing the host there finds it without relying on
    /// the icon theme or on the host rescanning icon directories.
    fn icon_theme_path(&self) -> String {
        std::env::current_exe()
            .ok()
            .and_then(|exe| Some(exe.parent()?.parent()?.join("share/icons")))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
    fn activate(&mut self, x: i32, y: i32) {
        // Some hosts send 0,0 when they do not know the position.
        self.toggle(((x, y) != (0, 0)).then_some((x, y)));
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![
            StandardItem {
                label: "Show VMs".into(),
                activate: Box::new(|t: &mut Self| t.toggle(None)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|t: &mut Self| {
                    if let Some(mut c) = t.popup.take() {
                        let _ = c.kill();
                    }
                    std::process::exit(0)
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let is_popup = args.first().is_some_and(|a| a == "popup");
    if is_popup {
        args.remove(0);
    }
    let opts = match options::parse(&args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("vm-tray: {e}\n{}", options::USAGE);
            std::process::exit(2);
        }
    };
    if is_popup {
        popup::run(opts);
        return;
    }
    let tray = Tray { args, popup: None };
    if let Err(e) = tray.spawn() {
        eprintln!("vm-tray: cannot register tray icon: {e}");
        std::process::exit(1);
    }
    loop {
        std::thread::park();
    }
}
