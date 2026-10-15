//! PNG snapshot of a part of the hidden WKWebView. wry has no snapshot API (ADR-0037).

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSDictionary, NSError, NSNumber};
use objc2_web_kit::{WKSnapshotConfiguration, WKWebView};

/// Snapshots have this many pixels per CSS pixel, whatever the screen scale.
const SCALE: f64 = crate::SCALE as f64;

/// Takes a snapshot of `rect`, in CSS pixels of the page, and calls `done` with the PNG bytes on
/// the main thread.
pub(crate) fn take(
    webview: &WKWebView,
    rect: (f64, f64, f64, f64),
    done: impl Fn(Result<Vec<u8>, String>) + 'static,
) {
    let (x, y, width, height) = rect;
    // `snapshotWidth` is in points and the image gets the window's backing scale, so this width
    // gives `SCALE` pixels per CSS pixel on any screen.
    let backing = webview
        .window()
        .map(|window| window.backingScaleFactor())
        .unwrap_or(SCALE);
    let block = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
        // SAFETY: WebKit passes either a valid image or a valid error, each possibly nil.
        let (image, error) = unsafe { (image.as_ref(), error.as_ref()) };
        done(match (image, error) {
            (Some(image), _) => png(image),
            (None, Some(error)) => Err(error.localizedDescription().to_string()),
            (None, None) => Err("snapshot returned no image".into()),
        });
    });
    // SAFETY: called on the main thread with a configuration this function owns.
    unsafe {
        let config = WKSnapshotConfiguration::new(MainThreadMarker::from(webview));
        config.setRect(CGRect::new(CGPoint::new(x, y), CGSize::new(width, height)));
        config.setSnapshotWidth(Some(&NSNumber::new_f64(width * SCALE / backing)));
        webview.takeSnapshotWithConfiguration_completionHandler(Some(&config), &block);
    }
}

fn png(image: &NSImage) -> Result<Vec<u8>, String> {
    let tiff = image.TIFFRepresentation().ok_or("snapshot has no bitmap")?;
    let bitmap = NSBitmapImageRep::imageRepWithData(&tiff).ok_or("snapshot has no bitmap")?;
    // SAFETY: an empty properties dictionary is valid for PNG.
    let data = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
    .ok_or("PNG encoding failed")?;
    Ok(data.to_vec())
}
