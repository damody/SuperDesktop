use platform_win::common::windows_notification_events::WindowsNotificationEventSource;

fn main() {
    match WindowsNotificationEventSource::new() {
        Ok(source) => {
            let access = source.access_status().access;
            match source.snapshot() {
                Ok(batch) => println!(
                    "access={access:?} synchronized={} count={} skipped={}",
                    batch.status.synchronized,
                    batch.notifications.len(),
                    batch.skipped
                ),
                Err(_) => println!("access={access:?} synchronized=false enumeration=failed"),
            }
        }
        Err(_) => println!("access=Unavailable synchronized=false activation=failed"),
    }
}
