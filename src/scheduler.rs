//! Mappa e gestisce il rinnovo automatico in background per computer portatili (Laptop Mode).
//! Cerca l'iPhone a intervalli regolari; quando lo trova e rinnova, manda una notifica nativa
//! e si ferma per il resto della giornata.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

use crate::web::App;

const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub auto_renew: bool,
    pub interval_mins: u64,
    pub renew_days_threshold: i64,
    pub last_renewed_date: Option<String>,
    pub auto_start_windows: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            auto_renew: true,
            interval_mins: 30,
            renew_days_threshold: 3,
            last_renewed_date: None,
            auto_start_windows: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatus {
    pub config: SchedulerConfig,
    pub state_code: String,
    pub message: String,
    pub last_check_time: Option<u64>,
}

static CURRENT_STATUS: Mutex<Option<SchedulerStatus>> = Mutex::new(None);

pub fn get_status() -> SchedulerStatus {
    let mut g = CURRENT_STATUS.lock().unwrap();
    if let Some(status) = g.as_ref() {
        let mut s = status.clone();
        s.config = load_config();
        return s;
    }
    let cfg = load_config();
    let s = SchedulerStatus {
        config: cfg.clone(),
        state_code: "idle".into(),
        message: "Scheduler initialized.".into(),
        last_check_time: None,
    };
    *g = Some(s.clone());
    s
}

pub fn update_status(code: &str, msg: &str) {
    let mut g = CURRENT_STATUS.lock().unwrap();
    let cfg = load_config();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).ok();
    *g = Some(SchedulerStatus {
        config: cfg,
        state_code: code.to_string(),
        message: msg.to_string(),
        last_check_time: now,
    });
}

pub fn load_config() -> SchedulerConfig {
    if let Ok(raw) = std::fs::read_to_string(CONFIG_FILE) {
        if let Ok(cfg) = serde_json::from_str::<SchedulerConfig>(&raw) {
            return cfg;
        }
    }
    SchedulerConfig::default()
}

pub fn save_config(cfg: &SchedulerConfig) {
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(CONFIG_FILE, json);
    }
}

pub fn today_date_string() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let iso = crate::apple::iso8601_utc(now);
    iso.chars().take(10).collect()
}

pub fn is_windows_autostart_enabled() -> bool {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let path = Path::new(&appdata)
                .join(r"Microsoft\Windows\Start Menu\Programs\Startup\AltKeeper.lnk");
            return path.exists();
        }
    }
    false
}

pub fn set_windows_autostart(enable: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_str = exe_path.to_string_lossy().to_string();
        let dir_str = exe_path.parent().unwrap_or(Path::new(".")).to_string_lossy().to_string();

        let script = if enable {
            format!(
                r#"
                $ws = New-Object -ComObject WScript.Shell
                $lnk = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\AltKeeper.lnk"
                $s = $ws.CreateShortcut($lnk)
                $s.TargetPath = "{}"
                $s.Arguments = "--silent"
                $s.WorkingDirectory = "{}"
                $s.Save()
                "#,
                exe_str.replace('\\', "\\\\"),
                dir_str.replace('\\', "\\\\")
            )
        } else {
            r#"
            $lnk = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\AltKeeper.lnk"
            if (Test-Path $lnk) { Remove-Item $lnk -Force }
            "#.to_string()
        };

        let status = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err("Failed to update Windows startup shortcut".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = enable;
        Ok(())
    }
}

async fn probe_profiles(phone_addr: Option<SocketAddr>) -> Option<Vec<crate::profile::PhoneProfile>> {
    let (_via, mut link) = crate::phone::connect_auto(phone_addr, crate::PAIRING_FILE, |_: &str| {}).await.ok()?;
    crate::phone_profiles(&mut link).await.ok()
}

pub fn start(app: Arc<App>) {
    tokio::spawn(async move {
        // Wait 8 seconds after boot before first check
        tokio::time::sleep(Duration::from_secs(8)).await;

        loop {
            let mut cfg = load_config();
            if !cfg.auto_renew {
                update_status("disabled", "Auto-renewal is paused in settings.");
                tokio::time::sleep(Duration::from_secs(30)).await;
                continue;
            }

            let today = today_date_string();
            let already_done = cfg.last_renewed_date.as_deref() == Some(&today);

            if already_done {
                update_status("done_today", &format!("Everything renewed/checked for today ({}). Search paused.", today));
                tokio::time::sleep(Duration::from_secs(1800)).await;
                continue;
            }

            update_status("checking", "Searching for iPhone (USB / Wi-Fi)...");

            let found = probe_profiles(app.phone).await;

            match found {
                Some(profiles) => {
                    let account = crate::apple::read_account(Path::new("."));
                    let team = account.as_ref().and_then(|a| a.team_id.as_deref());

                    let mut expiring_count = 0;
                    if let Some(t) = team {
                        let mine: Vec<_> = profiles.iter().filter(|p| p.team_id.as_deref() == Some(t)).collect();
                        let latest = crate::profile::latest_by_bundle(mine);
                        for p in latest {
                            if let Some(days) = p.days_left() {
                                if days <= cfg.renew_days_threshold {
                                    expiring_count += 1;
                                }
                            }
                        }
                    }

                    if expiring_count > 0 {
                        update_status("renewing", &format!("iPhone connected! Renewing {expiring_count} expiring apps..."));
                        let mut say = |_: &str| {};
                        match crate::cmd_renew(app.phone, false, false, cfg.renew_days_threshold, &mut say).await {
                            Ok(_) => {
                                cfg.last_renewed_date = Some(today.clone());
                                save_config(&cfg);
                                update_status("done_today", &format!("Successfully renewed {expiring_count} apps! Done for today."));
                                crate::notify::send_notification(
                                    "AltKeeper",
                                    &format!("Successfully renewed {expiring_count} iPhone apps! Done for today."),
                                );
                            }
                            Err(e) => {
                                update_status("error", &format!("Renewal failed: {e}"));
                            }
                        }
                    } else {
                        // All apps on device are already valid and fresh!
                        cfg.last_renewed_date = Some(today.clone());
                        save_config(&cfg);
                        update_status("done_today", &format!("All apps have > {} days left. Done for today.", cfg.renew_days_threshold));
                    }
                }
                None => {
                    let mins = cfg.interval_mins.max(5);
                    update_status("waiting_phone", &format!("Waiting for iPhone (will check again in {mins} minutes)..."));
                }
            }

            let sleep_secs = cfg.interval_mins.max(5) * 60;
            tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
        }
    });
}
