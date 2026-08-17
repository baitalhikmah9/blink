//! Login item via LaunchAgent plist.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

const LABEL: &str = "com.blink.app";

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

fn current_user_uid() -> std::io::Result<String> {
    let out = std::process::Command::new("id").arg("-u").output()?;
    if !out.status.success() {
        return Err(std::io::Error::other("id -u failed"));
    }
    String::from_utf8(out.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|_| std::io::Error::other("id -u output was not UTF-8"))
}

fn gui_domain() -> std::io::Result<String> {
    let uid = current_user_uid()?;
    Ok(format!("gui/{uid}"))
}

fn service_target() -> std::io::Result<String> {
    let uid = current_user_uid()?;
    Ok(format!("gui/{uid}/{LABEL}"))
}

fn xml_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&apos;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

fn plist_content(exe: &Path) -> String {
    let exe_str = exe.to_string_lossy();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <false/>
</dict>
</plist>
"#,
        xml_escape(&exe_str)
    )
}

fn launchctl_status(args: &[&str]) -> std::io::Result<std::process::ExitStatus> {
    std::process::Command::new("launchctl")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
}

/// True if launchctl reports the service as loaded in the user GUI domain.
/// Returns Ok(false) when the service is not found; Err on other probe failures.
pub fn is_enabled() -> std::io::Result<bool> {
    let target = service_target()?;
    let status = launchctl_status(&["print", &target])?;
    match status.code() {
        Some(0) => Ok(true),
        Some(113) => Ok(false),
        _ => Err(std::io::Error::other(format!(
            "launchctl print failed: {status}"
        ))),
    }
}

fn write_plist_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("plist.tmp");
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn set_enabled(enabled: bool) -> std::io::Result<()> {
    let path = agent_path().ok_or_else(|| std::io::Error::other("HOME unset"))?;
    let target = service_target()?;

    let status = launchctl_status(&["bootout", &target])?;
    match status.code() {
        Some(0) | Some(3) => {}
        _ => {
            return Err(std::io::Error::other(format!(
                "launchctl bootout failed: {status}"
            )));
        }
    }

    if !enabled {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }

    let exe = exe_path().ok_or_else(|| std::io::Error::other("cannot resolve exe path"))?;
    let desired = plist_content(&exe);
    write_plist_atomic(&path, &desired)?;

    let domain = gui_domain()?;
    let status = launchctl_status(&["bootstrap", &domain, path.to_str().unwrap_or_default()])?;
    if !status.success() {
        let _ = std::fs::remove_file(&path);
        return Err(std::io::Error::other(format!(
            "launchctl bootstrap failed: {status}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn xml_escapes_special_chars() {
        let s = "/Users/foo & bar/Blink!.app/Contents/MacOS/blink";
        assert_eq!(
            xml_escape(s),
            "/Users/foo &amp; bar/Blink!.app/Contents/MacOS/blink"
        );
        assert_eq!(xml_escape("a < b > c"), "a &lt; b &gt; c");
        assert_eq!(xml_escape("\"x\" 'y'"), "&quot;x&quot; &apos;y&apos;");
    }

    #[test]
    fn plist_content_includes_escaped_exe() {
        let exe = Path::new("/Users/test & user/bin/blink");
        let content = plist_content(exe);
        assert!(content.contains("&amp;"));
        assert!(content.contains("<string>/Users/test &amp; user/bin/blink</string>"));
    }

    #[test]
    fn atomic_plist_round_trip() {
        let tmp = env::temp_dir().join(format!("blink-atomic-plist-{}", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(tmp.with_extension("plist.tmp"));
        let content = plist_content(Path::new("/tmp/blink"));
        write_plist_atomic(&tmp, &content).unwrap();
        assert_eq!(std::fs::read_to_string(&tmp).unwrap(), content);
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(tmp.with_extension("plist.tmp"));
    }
}
