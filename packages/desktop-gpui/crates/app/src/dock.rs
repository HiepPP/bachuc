//! The dock badge and the dock fire. GPUI has no API for either, so this uses `NSDockTile`.

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

/// Whether the app is frontmost. Off the main thread it reads as active, so no fire starts.
#[cfg(target_os = "macos")]
pub fn app_is_active() -> bool {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    MainThreadMarker::new()
        .is_none_or(|main_thread| NSApplication::sharedApplication(main_thread).isActive())
}

#[cfg(not(target_os = "macos"))]
pub fn app_is_active() -> bool {
    true
}

/// Pixels of the tile image; the dock shows it at 128 points or less.
#[cfg(target_os = "macos")]
const TILE_PIXELS: isize = 256;

#[cfg(target_os = "macos")]
thread_local! {
    static FIRE_VIEW: std::cell::RefCell<Option<objc2::rc::Retained<objc2_app_kit::NSImageView>>> =
        const { std::cell::RefCell::new(None) };
    /// The app icon scaled to the tile once, so a frame does not scale the full icon again.
    static TILE_ICON: std::cell::RefCell<Option<objc2::rc::Retained<objc2_app_kit::NSImage>>> =
        const { std::cell::RefCell::new(None) };
}

/// Draws the app icon with `flames` over it in the dock tile. `flames` holds premultiplied
/// RGBA rows of `side` pixels, top row first. Call it on the main thread.
#[cfg(target_os = "macos")]
pub fn show_fire(flames: &[u8], side: usize) {
    use objc2::{AnyThread as _, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage, NSImageView};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(main_thread);
    let (Some(icon), Some(fire), Some(tile_rep)) =
        (tile_icon(&app), bitmap(side as isize), bitmap(TILE_PIXELS))
    else {
        return;
    };
    // SAFETY: `bitmap` gives a meshed 32-bit RGBA buffer of exactly `side * side * 4` bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(flames.as_ptr(), fire.bitmapData(), side * side * 4);
    }
    let fire_image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(side as f64, side as f64));
    fire_image.addRepresentation(&fire);
    if !draw_tile(&tile_rep, |rect| {
        icon.drawInRect(rect);
        fire_image.drawInRect(rect);
    }) {
        return;
    }

    let pixels = TILE_PIXELS as f64;
    let image = NSImage::initWithSize(NSImage::alloc(), NSSize::new(pixels, pixels));
    image.addRepresentation(&tile_rep);
    let tile = app.dockTile();
    FIRE_VIEW.with_borrow_mut(|view| match view {
        Some(view) => view.setImage(Some(&image)),
        None => {
            let created = NSImageView::imageViewWithImage(&image, main_thread);
            created.setFrame(NSRect::new(NSPoint::new(0., 0.), tile.size()));
            tile.setContentView(Some(&created));
            *view = Some(created);
        }
    });
    tile.display();
}

#[cfg(not(target_os = "macos"))]
pub fn show_fire(_flames: &[u8], _side: usize) {}

/// Puts the plain app icon back in the dock tile. Call it on the main thread.
#[cfg(target_os = "macos")]
pub fn clear_fire() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    FIRE_VIEW.with_borrow_mut(|view| *view = None);
    let tile = NSApplication::sharedApplication(main_thread).dockTile();
    tile.setContentView(None);
    tile.display();
}

#[cfg(not(target_os = "macos"))]
pub fn clear_fire() {}

/// The app icon at tile size, drawn once and kept.
#[cfg(target_os = "macos")]
fn tile_icon(
    app: &objc2_app_kit::NSApplication,
) -> Option<objc2::rc::Retained<objc2_app_kit::NSImage>> {
    use objc2::AnyThread as _;
    use objc2_app_kit::NSImage;
    use objc2_foundation::NSSize;

    if let Some(icon) = TILE_ICON.with_borrow(Clone::clone) {
        return Some(icon);
    }
    let (source, rep) = (app.applicationIconImage()?, bitmap(TILE_PIXELS)?);
    if !draw_tile(&rep, |rect| source.drawInRect(rect)) {
        return None;
    }
    let pixels = TILE_PIXELS as f64;
    let icon = NSImage::initWithSize(NSImage::alloc(), NSSize::new(pixels, pixels));
    icon.addRepresentation(&rep);
    TILE_ICON.with_borrow_mut(|cached| *cached = Some(icon.clone()));
    Some(icon)
}

/// Runs `draw` with `rep` as the current context and the full tile as its rect. False when
/// AppKit gives no context for `rep`.
#[cfg(target_os = "macos")]
fn draw_tile(
    rep: &objc2_app_kit::NSBitmapImageRep,
    draw: impl FnOnce(objc2_foundation::NSRect),
) -> bool {
    use objc2_app_kit::{NSGraphicsContext, NSImageInterpolation};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    let Some(context) = NSGraphicsContext::graphicsContextWithBitmapImageRep(rep) else {
        return false;
    };
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&context));
    context.setImageInterpolation(NSImageInterpolation::High);
    let pixels = TILE_PIXELS as f64;
    draw(NSRect::new(
        NSPoint::new(0., 0.),
        NSSize::new(pixels, pixels),
    ));
    context.flushGraphics();
    NSGraphicsContext::restoreGraphicsState_class();
    true
}

/// An empty RGBA bitmap of `side` pixels with premultiplied alpha.
#[cfg(target_os = "macos")]
fn bitmap(side: isize) -> Option<objc2::rc::Retained<objc2_app_kit::NSBitmapImageRep>> {
    use objc2::AnyThread as _;
    use objc2_app_kit::{NSBitmapImageRep, NSDeviceRGBColorSpace};

    // SAFETY: null planes make AppKit allocate the pixels; the color space is AppKit's own.
    unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            side,
            side,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            side * 4,
            32,
        )
    }
}
