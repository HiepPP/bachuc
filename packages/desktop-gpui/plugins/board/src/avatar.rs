//! The bot avatars of `client/avatar.tsx`: 20 PNGs, one per agent by an FNV-1a hash of its id.
//! The PNGs stay in the frozen React client's folder.

use gpui_kit::{AnyElement, Image, ImageFormat, IntoElement as _, Pixels, Styled as _, img};
use std::sync::{Arc, LazyLock};

macro_rules! bots {
    ($($n:literal),*) => {
        [$(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../hiep-plugins/plugins/board/client/assets/bots/bot-",
            $n,
            ".png"
        )) as &[u8]),*]
    };
}

const BOTS: [&[u8]; 20] = bots!(
    "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15", "16",
    "17", "18", "19", "20"
);

static AVATARS: LazyLock<Vec<Arc<Image>>> = LazyLock::new(|| {
    BOTS.iter()
        .map(|bytes| Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec())))
        .collect()
});

/// The avatar of an agent: stable across refreshes and hosts. FNV-1a over the first UTF-16
/// unit of each character, as `avatarIndex` hashes `charCodeAt(0)` per code point.
pub(crate) fn avatar_index(agent_id: &str) -> usize {
    let mut hash: u32 = 2_166_136_261;
    for char in agent_id.chars() {
        let mut units = [0u16; 2];
        let unit = char.encode_utf16(&mut units)[0];
        hash = (hash ^ u32::from(unit)).wrapping_mul(16_777_619);
    }
    hash as usize % BOTS.len()
}

pub(crate) fn agent_avatar(agent_id: &str, size: Pixels) -> AnyElement {
    img(AVATARS[avatar_index(agent_id)].clone())
        .flex_none()
        .size(size)
        .rounded(size * 0.22)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_avatar_index_is_stable_and_matches_the_electron_hash() {
        // Values from `avatarIndex` in Node.
        for (id, index) in [
            ("", 1),
            ("a", 0),
            ("agent-1", 4),
            ("0f8e1c2a-7b3d-4e5f-9a1b-2c3d4e5f6a7b", 4),
            ("élan😀", 2),
        ] {
            assert_eq!(avatar_index(id), index, "{id}");
            assert_eq!(avatar_index(id), avatar_index(id));
        }
        for n in 0..500 {
            assert!(avatar_index(&format!("agent-{n}")) < 20);
        }
    }

    #[test]
    fn every_avatar_is_a_png() {
        for bytes in BOTS {
            assert!(bytes.starts_with(b"\x89PNG"));
        }
    }
}
