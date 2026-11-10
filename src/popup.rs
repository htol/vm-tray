use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use virt::connect::Connect;

use crate::options::Options;
use crate::vm::{self, Action, State, Vm};

const CSS: &str = "
window { background: #2e3440; color: #eceff4; border: 1px solid #4c566a; border-radius: 8px; }
.uri { color: #88c0d0; font-weight: bold; margin-top: 4px; }
.name { font-weight: bold; }
.running { color: #a3be8c; }
.paused { color: #ebcb8b; }
.off { color: #616e88; }
.error { color: #bf616a; }
button { background: #3b4252; color: #eceff4; border: none; box-shadow: none; padding: 2px 10px; min-height: 0; }
button:hover { background: #434c5e; }
button:disabled { color: #616e88; background: #3b4252; }
";

/// One libvirt connection and its last observed VM list.
struct Source {
    uri: String,
    conn: Result<Connect, String>,
}

type Snapshot = Vec<(String, Result<Vec<Vm>, String>)>;

pub fn run(opts: Options) {
    // A few rows of widgets do not need the GPU renderer; cairo uses far less
    // memory (measured: ~62 MB RSS vs ~155 MB with Vulkan).
    if std::env::var_os("GSK_RENDERER").is_none() {
        // SAFETY: no other threads exist yet.
        unsafe { std::env::set_var("GSK_RENDERER", "cairo") };
    }
    let app = gtk::Application::builder()
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| build(app, &opts));
    app.run_with_args::<&str>(&[]);
}

fn build(app: &gtk::Application, opts: &Options) {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("no display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Virtual machines")
        .resizable(false)
        .build();

    // Close on Escape and when focus moves elsewhere.
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(glib::clone!(
        #[weak]
        window,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, _| {
            if key == gdk::Key::Escape {
                window.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    ));
    window.add_controller(keys);
    window.connect_is_active_notify(|w| {
        if !w.is_active() {
            w.close();
        }
    });

    let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
    root.set_margin_top(10);
    root.set_margin_bottom(10);
    root.set_margin_start(12);
    root.set_margin_end(12);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let status = gtk::Label::new(None);
    status.add_css_class("error");
    status.set_wrap(true);
    status.set_max_width_chars(48);
    status.set_xalign(0.0);
    status.set_visible(false);
    root.append(&list);
    root.append(&status);
    window.set_child(Some(&root));

    let sources: Rc<Vec<Source>> = Rc::new(
        opts.uris
            .iter()
            .map(|uri| Source {
                uri: uri.clone(),
                conn: Connect::open(Some(uri)).map_err(|e| e.to_string()),
            })
            .collect(),
    );
    let last: Rc<RefCell<Option<Snapshot>>> = Rc::new(RefCell::new(None));

    let refresh = {
        let (list, status) = (list.clone(), status.clone());
        move || {
            let snap: Snapshot = sources
                .iter()
                .map(|s| {
                    let vms = match &s.conn {
                        Ok(c) => vm::list(c),
                        Err(e) => Err(e.clone()),
                    };
                    (s.uri.clone(), vms)
                })
                .collect();
            if last.borrow().as_ref() != Some(&snap) {
                render(&list, &status, &snap);
                *last.borrow_mut() = Some(snap);
            }
        }
    };
    refresh();
    // After the first render, so the content width is known.
    if gtk4_layer_shell::is_supported() {
        place(&window, opts);
    }
    // Polling runs only while the popup is open, so it costs nothing otherwise.
    glib::timeout_add_local(Duration::from_secs(1), move || {
        refresh();
        glib::ControlFlow::Continue
    });

    window.present();
}

const GAP: i32 = 8;

/// Turns the window into a layer surface. With a click position it opens on
/// that monitor, centered on the click and at the nearer vertical edge (where
/// the panel is); otherwise in the `--anchor` corner.
fn place(window: &gtk::ApplicationWindow, opts: &Options) {
    window.init_layer_shell();
    window.set_namespace(Some("vm-tray"));
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    if let Some((x, y)) = opts.at
        && let Some(mon) = monitor_at(x, y)
    {
        let g = mon.geometry();
        let (_, width, _, _) = window.measure(gtk::Orientation::Horizontal, -1);
        let v = if y - g.y() > g.height() / 2 {
            Edge::Bottom
        } else {
            Edge::Top
        };
        let left = (x - g.x() - width / 2).clamp(GAP, (g.width() - width - GAP).max(GAP));
        window.set_monitor(Some(&mon));
        window.set_anchor(v, true);
        window.set_margin(v, GAP);
        window.set_anchor(Edge::Left, true);
        window.set_margin(Edge::Left, left);
        return;
    }

    let (v, h) = opts.anchor.split_once('-').expect("validated in parse");
    let v = if v == "top" { Edge::Top } else { Edge::Bottom };
    let h = if h == "left" { Edge::Left } else { Edge::Right };
    for edge in [v, h] {
        window.set_anchor(edge, true);
        window.set_margin(edge, GAP);
    }
}

fn monitor_at(x: i32, y: i32) -> Option<gdk::Monitor> {
    let monitors = gdk::Display::default()?.monitors();
    (0..monitors.n_items())
        .filter_map(|i| monitors.item(i)?.downcast::<gdk::Monitor>().ok())
        .find(|m| {
            let g = m.geometry();
            (g.x()..g.x() + g.width()).contains(&x) && (g.y()..g.y() + g.height()).contains(&y)
        })
}

fn render(list: &gtk::Box, status: &gtk::Label, snap: &Snapshot) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for (uri, vms) in snap {
        let header = gtk::Label::new(Some(uri));
        header.add_css_class("uri");
        header.set_xalign(0.0);
        list.append(&header);

        let vms = match vms {
            Ok(v) => v,
            Err(e) => {
                let l = gtk::Label::new(Some(e));
                l.add_css_class("error");
                l.set_wrap(true);
                l.set_max_width_chars(48);
                l.set_xalign(0.0);
                list.append(&l);
                continue;
            }
        };
        if vms.is_empty() {
            let l = gtk::Label::new(Some("no VMs"));
            l.add_css_class("off");
            l.set_xalign(0.0);
            list.append(&l);
            continue;
        }

        let grid = gtk::Grid::builder()
            .column_spacing(12)
            .row_spacing(4)
            .build();
        for (row, vm) in vms.iter().enumerate() {
            let row = row as i32;
            let name = gtk::Label::new(Some(&vm.name));
            name.add_css_class("name");
            name.set_xalign(0.0);
            name.set_hexpand(true);
            grid.attach(&name, 0, row, 1, 1);

            let state = gtk::Label::new(Some(vm.state.label()));
            state.add_css_class(match vm.state {
                State::Running => "running",
                State::Paused => "paused",
                State::Off => "off",
            });
            state.set_xalign(0.0);
            grid.attach(&state, 1, row, 1, 1);

            let (label, action) = match vm.state {
                State::Off => ("Start", Action::Start),
                State::Running => ("Stop", Action::Stop),
                State::Paused => ("Resume", Action::Resume),
            };
            let toggle = gtk::Button::with_label(label);
            toggle.connect_clicked(glib::clone!(
                #[strong]
                uri,
                #[strong(rename_to = name)]
                vm.name,
                #[weak]
                status,
                move |btn| {
                    // Stays disabled until the state changes and the list is rebuilt.
                    btn.set_sensitive(false);
                    status.set_visible(false);
                    let (uri, name) = (uri.clone(), name.clone());
                    let btn = btn.clone();
                    let status = status.clone();
                    glib::spawn_future_local(async move {
                        let res = gio::spawn_blocking(move || vm::run(&uri, &name, action)).await;
                        if let Ok(Err(e)) = res {
                            status.set_text(&e);
                            status.set_visible(true);
                            btn.set_sensitive(true);
                        }
                    });
                }
            ));
            grid.attach(&toggle, 2, row, 1, 1);

            let viewer = gtk::Button::with_label("Viewer");
            viewer.set_sensitive(vm.state != State::Off);
            viewer.connect_clicked(glib::clone!(
                #[strong]
                uri,
                #[strong(rename_to = name)]
                vm.name,
                #[weak]
                status,
                move |_| {
                    if let Err(e) = vm::open_viewer(&uri, &name) {
                        status.set_text(&e);
                        status.set_visible(true);
                    }
                }
            ));
            grid.attach(&viewer, 3, row, 1, 1);
        }
        list.append(&grid);
    }
}
