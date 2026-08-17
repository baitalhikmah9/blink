//! Login item via LaunchAgent plist.

use std::path::PathBuf;

const LABEL: &str = "com.blinkcue.app";

fn agent_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist")),
    )
}

fn exe_path() -> Option<PathBuf> {
    std::env::current_exe().ok()?.canonicalize().ok()
}

pub fn is_enabled() -> bool {
    agent_path().map(|p| p.exists()).unwrap_or(false)
}

pub fn set_enabled(enabled: bool) -> std::io::Result<()> {
    let path = agent_path().ok_or_else(|| std::io::Error::other("HOME unset"))?;
    if !enabled {
        if path.exists() {
            let _ = std::process::Command::new("launchctl")
                .args(["unload", path.to_str().unwrap_or_default()])
                .status();
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }

    let exe = exe_path().ok_or_else(|| std::io::Error::other("cannot resolve exe path"))?;
    let exe_str = exe.to_string_lossy();
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe_str}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <false/>
</dict>
</plist>
"#
    );

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, plist)?;
    let _ = std::process::Command::new("launchctl")
        .args(["load", path.to_str().unwrap_or_default()])
        .status();
    Ok(())
}
