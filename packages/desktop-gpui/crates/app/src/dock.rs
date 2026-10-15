//! The dock badge. GPUI has no API for it, so this sets `NSDockTile` `badgeLabel`.

/// Shows `label` on the dock icon, or clears the badge when `label` is none. Call it on the
/// main thread; elsewhere it does nothing.
#[cfg(target_os = "macos")]
pub fn set_badge(label: Option<&str>) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    use objc2_foundation::NSString;

    let Some(main_thread) = MainThreadMarker::new() else {
        log::warn!("dock badge set off the main thread");
        return;
    };
    let tile = NSApplication::sharedApplication(main_thread).dockTile();
    match label {
        Some(text) => tile.setBadgeLabel(Some(&NSString::from_str(text))),
        None => tile.setBadgeLabel(None),
    }
    log::info!(
        "dock badge set to {label:?}, reads back {:?}",
        tile.badgeLabel().map(|text| text.to_string())
    );
}

#[cfg(not(target_os = "macos"))]
pub fn set_badge(_label: Option<&str>) {}
