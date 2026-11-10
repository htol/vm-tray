//! Command-line options for the tray and `vm-tray popup`.

pub const USAGE: &str = "usage: vm-tray [popup] [--connect URI[,URI...]] [--anchor top-right|top-left|bottom-right|bottom-left] [--at X,Y]";

pub struct Options {
    pub uris: Vec<String>,
    pub anchor: String,
    /// Tray click position in global coordinates; the popup opens next to it.
    pub at: Option<(i32, i32)>,
}

pub fn parse(args: &[String]) -> Result<Options, String> {
    let mut uris = "qemu:///system,qemu:///session".to_string();
    let mut anchor = "bottom-right".to_string();
    let mut at = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let v = it.next().ok_or_else(|| format!("{a}: missing value"))?;
        match a.as_str() {
            "--connect" => uris = v.clone(),
            "--anchor" => anchor = v.clone(),
            "--at" => {
                let (x, y) = v
                    .split_once(',')
                    .and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?)))
                    .ok_or_else(|| format!("bad --at: {v}"))?;
                at = Some((x, y));
            }
            _ => return Err(format!("unknown argument: {a}")),
        }
    }
    if !matches!(
        anchor.as_str(),
        "top-right" | "top-left" | "bottom-right" | "bottom-left"
    ) {
        return Err(format!("bad --anchor: {anchor}"));
    }
    let uris = uris
        .split(',')
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map(String::from)
        .collect();
    Ok(Options { uris, anchor, at })
}
