//! File icons of the Files tree and the Changes tree: the colored material icons of the Electron
//! explorer (docs/file-icons.md), looked up by extension. `file_icon_data.rs` is generated.

use crate::file_icon_data::{EXTENSIONS, ICONS};
use gpui_kit::{Image, ImageFormat};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

const DEFAULT_ICON: &str = "_default";

/// The icon name for `file_name`, as `getRawFileIconSvg` picks it: by the text after the last
/// dot, in lower case, and the generic file icon when there is none or it has no icon.
pub fn icon_name(file_name: &str) -> &'static str {
    let Some((_, extension)) = file_name.rsplit_once('.') else {
        return DEFAULT_ICON;
    };
    if extension.is_empty() {
        return DEFAULT_ICON;
    }
    let extension = extension.to_lowercase();
    EXTENSIONS
        .binary_search_by(|(known, _)| (*known).cmp(extension.as_str()))
        .map_or(DEFAULT_ICON, |found| EXTENSIONS[found].1)
}

fn images() -> &'static HashMap<&'static str, Arc<Image>> {
    static IMAGES: OnceLock<HashMap<&'static str, Arc<Image>>> = OnceLock::new();
    IMAGES.get_or_init(|| {
        ICONS
            .iter()
            .map(|(name, svg)| {
                (
                    *name,
                    Arc::new(Image::from_bytes(ImageFormat::Svg, svg.as_bytes().to_vec())),
                )
            })
            .collect()
    })
}

/// The icon image for `file_name`. GPUI draws an SVG image at twice the layout size, so it stays
/// sharp on a retina display.
pub fn file_icon(file_name: &str) -> Arc<Image> {
    let images = images();
    images
        .get(icon_name(file_name))
        .or_else(|| images.get(DEFAULT_ICON))
        .cloned()
        .expect("the generated table has the default icon")
}
