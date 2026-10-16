//! Lucide icons for the GPUI client, the same set and drawing the Electron app uses (ADR-0043).
//! `codegen/generate-icons.mjs` writes `assets/*.svg` and `names.rs` from the icons that
//! `packages/app/src` imports; run it again when the app imports a new one.

// Generated, so rustfmt must not rewrap it: a second generator run would then change the file.
#[rustfmt::skip]
mod names;

use bachuc_theme::metrics::ICON_MD;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::{AssetSource, Result, SharedString, px};
use std::borrow::Cow;

pub use names::IconName;

/// The asset source for the app: it serves the generated SVGs and falls through to the gpui-kit
/// assets, so gpui-kit components keep their own icons (menu, select, dialog close).
/// Register it with `gpui_kit::application().with_assets(bachuc_icons::Assets)`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Assets;

fn bundled(path: &str) -> Option<&'static [u8]> {
    names::SVGS
        .binary_search_by(|(name, _)| (*name).cmp(path))
        .ok()
        .map(|found| names::SVGS[found].1)
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match bundled(path) {
            Some(bytes) => Ok(Some(Cow::Borrowed(bytes))),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths: Vec<SharedString> = names::SVGS
            .iter()
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| SharedString::from(*name))
            .collect();
        paths.extend(gpui_kit::assets::Assets.list(path)?);
        Ok(paths)
    }
}

/// An icon element, 16 px by default. Change the size with `with_size` and the color with
/// `text_color`.
pub fn icon(name: IconName) -> Icon {
    Icon::default().path(name.path()).with_size(px(ICON_MD))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_name_loads_a_svg() {
        for name in IconName::ALL {
            let bytes = Assets
                .load(name.path())
                .unwrap_or_else(|error| panic!("{name:?}: {error}"))
                .unwrap_or_else(|| panic!("{name:?}: no data"));
            let text = std::str::from_utf8(&bytes).unwrap();
            assert!(text.starts_with("<svg"), "{name:?}: {text}");
            assert!(text.contains("viewBox=\"0 0 24 24\""), "{name:?}");
            assert!(text.contains("stroke=\"currentColor\""), "{name:?}");
            assert!(text.trim_end().ends_with("</svg>"), "{name:?}");
        }
    }

    #[test]
    fn nodes_other_than_path_survive() {
        let text = String::from_utf8(
            Assets
                .load(IconName::GitBranch.path())
                .unwrap()
                .unwrap()
                .into_owned(),
        )
        .unwrap();
        assert_eq!(text.matches("<circle ").count(), 2, "{text}");
        assert_eq!(text.matches("<line ").count(), 1, "{text}");
        assert_eq!(text.matches("<path ").count(), 1, "{text}");
    }

    #[test]
    fn gpui_kit_asset_paths_fall_through() {
        let kit_path = "icons/check.svg";
        assert!(
            bundled(kit_path).is_none(),
            "the test path must come from gpui-kit"
        );
        let bytes = Assets.load(kit_path).unwrap().unwrap();
        assert!(!bytes.is_empty());
        assert!(
            Assets
                .list("icons/")
                .unwrap()
                .iter()
                .any(|path| path == kit_path)
        );
    }

    #[test]
    fn an_unknown_path_is_an_error_like_gpui_kit() {
        assert!(Assets.load("lucide/not-an-icon.svg").is_err());
    }

    #[test]
    fn the_table_is_sorted_for_binary_search() {
        assert!(names::SVGS.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }
}
