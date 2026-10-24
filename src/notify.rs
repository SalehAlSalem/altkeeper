//! Notifiche native di sistema (Windows Toast Notifications).

pub fn send_notification(title: &str, body: &str) {
    #[cfg(windows)]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let script = format!(
            r#"
            [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
            [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
            $xml = @"
            <toast>
                <visual>
                    <binding template="ToastGeneric">
                        <text>{}</text>
                        <text>{}</text>
                    </binding>
                </visual>
            </toast>
"@
            $doc = New-Object Windows.Data.Xml.Dom.XmlDocument
            $doc.LoadXml($xml)
            $toast = [Windows.UI.Notifications.ToastNotification]::new($doc)
            [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("AltKeeper").Show($toast)
            "#,
            title.replace('"', "`\""),
            body.replace('"', "`\"")
        );

        let _ = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        // On Linux, fallback to notify-send if present
        let _ = std::process::Command::new("notify-send")
            .args([title, body])
            .spawn();
    }
}
