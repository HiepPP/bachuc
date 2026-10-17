//! The eight syntax themes of `packages/highlight/src/themes.ts` with the palettes of
//! `packages/highlight/src/colors.ts`. A theme with one palette uses it in both app themes.

use gpui_kit::{Hsla, Rgba};

use crate::Scheme;

/// A syntax theme id of `SyntaxThemeId` in `themes.ts`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SyntaxThemeId {
    Github,
    Catppuccin,
    Dracula,
    TokyoNight,
    #[default]
    One,
    Nord,
    Gruvbox,
    Solarized,
}

impl SyntaxThemeId {
    /// In the order of `SYNTAX_THEME_OPTIONS`.
    pub const ALL: [Self; 8] = [
        Self::Github,
        Self::Catppuccin,
        Self::Dracula,
        Self::TokyoNight,
        Self::One,
        Self::Nord,
        Self::Gruvbox,
        Self::Solarized,
    ];

    /// The id that Electron stores.
    pub fn id(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Catppuccin => "catppuccin",
            Self::Dracula => "dracula",
            Self::TokyoNight => "tokyo-night",
            Self::One => "one",
            Self::Nord => "nord",
            Self::Gruvbox => "gruvbox",
            Self::Solarized => "solarized",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Github => "GitHub",
            Self::Catppuccin => "Catppuccin",
            Self::Dracula => "Dracula",
            Self::TokyoNight => "Tokyo Night",
            Self::One => "One",
            Self::Nord => "Nord",
            Self::Gruvbox => "Gruvbox",
            Self::Solarized => "Solarized",
        }
    }

    /// `None` for an id that is not one of the eight. Use `unwrap_or_default` for the fallback.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.id() == id)
    }
}

/// One color per highlight role of `colors.ts`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyntaxColors {
    pub keyword: Hsla,
    pub comment: Hsla,
    pub string: Hsla,
    pub number: Hsla,
    pub literal: Hsla,
    pub function: Hsla,
    pub definition: Hsla,
    pub class: Hsla,
    pub r#type: Hsla,
    pub tag: Hsla,
    pub attribute: Hsla,
    pub property: Hsla,
    pub variable: Hsla,
    pub operator: Hsla,
    pub punctuation: Hsla,
    pub regexp: Hsla,
    pub escape: Hsla,
    pub meta: Hsla,
    pub heading: Hsla,
    pub link: Hsla,
}

/// The role names of `HighlightStyle`, in the order of [`SyntaxColors::entries`].
pub const ROLES: [&str; 20] = [
    "keyword",
    "comment",
    "string",
    "number",
    "literal",
    "function",
    "definition",
    "class",
    "type",
    "tag",
    "attribute",
    "property",
    "variable",
    "operator",
    "punctuation",
    "regexp",
    "escape",
    "meta",
    "heading",
    "link",
];

impl SyntaxColors {
    /// Each role name with its color, in the order of [`ROLES`].
    pub fn entries(&self) -> [(&'static str, Hsla); 20] {
        let colors = [
            self.keyword,
            self.comment,
            self.string,
            self.number,
            self.literal,
            self.function,
            self.definition,
            self.class,
            self.r#type,
            self.tag,
            self.attribute,
            self.property,
            self.variable,
            self.operator,
            self.punctuation,
            self.regexp,
            self.escape,
            self.meta,
            self.heading,
            self.link,
        ];
        std::array::from_fn(|index| (ROLES[index], colors[index]))
    }

    fn from_hex(hex: [u32; 20]) -> Self {
        let [
            keyword,
            comment,
            string,
            number,
            literal,
            function,
            definition,
            class,
            r#type,
            tag,
            attribute,
            property,
            variable,
            operator,
            punctuation,
            regexp,
            escape,
            meta,
            heading,
            link,
        ] = hex.map(color);
        Self {
            keyword,
            comment,
            string,
            number,
            literal,
            function,
            definition,
            class,
            r#type,
            tag,
            attribute,
            property,
            variable,
            operator,
            punctuation,
            regexp,
            escape,
            meta,
            heading,
            link,
        }
    }
}

fn color(hex: u32) -> Hsla {
    Rgba {
        r: ((hex >> 16) & 0xff) as f32 / 255.,
        g: ((hex >> 8) & 0xff) as f32 / 255.,
        b: (hex & 0xff) as f32 / 255.,
        a: 1.,
    }
    .into()
}

/// The palette of `theme` for the light or the dark app theme.
pub fn syntax_colors_for(theme: SyntaxThemeId, scheme: Scheme) -> SyntaxColors {
    let dark = scheme == Scheme::Dark;
    let hex = match theme {
        SyntaxThemeId::Github => {
            if dark {
                GITHUB_DARK
            } else {
                GITHUB_LIGHT
            }
        }
        SyntaxThemeId::Catppuccin => expand(if dark {
            CATPPUCCIN_MOCHA
        } else {
            CATPPUCCIN_LATTE
        }),
        SyntaxThemeId::Dracula => expand(DRACULA),
        SyntaxThemeId::TokyoNight => expand(if dark { TOKYO_NIGHT } else { TOKYO_DAY }),
        SyntaxThemeId::One => expand(if dark { ONE_DARK } else { ONE_LIGHT }),
        SyntaxThemeId::Nord => expand(if dark { NORD_DARK } else { NORD_LIGHT }),
        SyntaxThemeId::Gruvbox => expand(if dark { GRUVBOX_DARK } else { GRUVBOX_LIGHT }),
        SyntaxThemeId::Solarized => expand(if dark {
            SOLARIZED_DARK
        } else {
            SOLARIZED_LIGHT
        }),
    };
    SyntaxColors::from_hex(hex)
}

/// `RolePalette` in `themes.ts`.
struct RolePalette {
    base: u32,
    keyword: u32,
    comment: u32,
    string: u32,
    number: u32,
    function: u32,
    r#type: u32,
    tag: u32,
    attribute: u32,
    operator: u32,
}

/// `expandRolePalette` in `themes.ts`, in the order of [`ROLES`].
fn expand(r: RolePalette) -> [u32; 20] {
    [
        r.keyword,   // keyword
        r.comment,   // comment
        r.string,    // string
        r.number,    // number
        r.number,    // literal
        r.function,  // function
        r.function,  // definition
        r.r#type,    // class
        r.r#type,    // type
        r.tag,       // tag
        r.attribute, // attribute
        r.attribute, // property
        r.base,      // variable
        r.operator,  // operator
        r.base,      // punctuation
        r.string,    // regexp
        r.number,    // escape
        r.comment,   // meta
        r.function,  // heading
        r.string,    // link
    ]
}

/// `darkHighlightColors` in `colors.ts`, in the order of [`ROLES`].
const GITHUB_DARK: [u32; 20] = [
    0xff7b72, 0x8b949e, 0xa5d6ff, 0x79c0ff, 0x79c0ff, 0xd2a8ff, 0xd2a8ff, 0xffa657, 0xff7b72,
    0x7ee787, 0x79c0ff, 0x79c0ff, 0xc9d1d9, 0x79c0ff, 0xc9d1d9, 0xa5d6ff, 0x79c0ff, 0x8b949e,
    0x79c0ff, 0xa5d6ff,
];

/// `lightHighlightColors` in `colors.ts`, in the order of [`ROLES`].
const GITHUB_LIGHT: [u32; 20] = [
    0xcf222e, 0x6e7781, 0x0a3069, 0x0550ae, 0x0550ae, 0x8250df, 0x8250df, 0x953800, 0xcf222e,
    0x116329, 0x0550ae, 0x0550ae, 0x24292f, 0x0550ae, 0x24292f, 0x0a3069, 0x0550ae, 0x6e7781,
    0x0550ae, 0x0a3069,
];

const CATPPUCCIN_LATTE: RolePalette = RolePalette {
    base: 0x4c4f69,
    keyword: 0x8839ef,
    comment: 0x8c8fa1,
    string: 0x40a02b,
    number: 0xfe640b,
    function: 0x1e66f5,
    r#type: 0xdf8e1d,
    tag: 0x8839ef,
    attribute: 0xdf8e1d,
    operator: 0x04a5e5,
};
const CATPPUCCIN_MOCHA: RolePalette = RolePalette {
    base: 0xcdd6f4,
    keyword: 0xcba6f7,
    comment: 0x9399b2,
    string: 0xa6e3a1,
    number: 0xfab387,
    function: 0x89b4fa,
    r#type: 0xf9e2af,
    tag: 0xcba6f7,
    attribute: 0xf9e2af,
    operator: 0x89dceb,
};

const DRACULA: RolePalette = RolePalette {
    base: 0xf8f8f2,
    keyword: 0xff79c6,
    comment: 0x6272a4,
    string: 0xf1fa8c,
    number: 0xbd93f9,
    function: 0x50fa7b,
    r#type: 0x8be9fd,
    tag: 0xff79c6,
    attribute: 0x50fa7b,
    operator: 0xff79c6,
};

const TOKYO_DAY: RolePalette = RolePalette {
    base: 0x3760bf,
    keyword: 0x9854f1,
    comment: 0x848cb5,
    string: 0x587539,
    number: 0xb15c00,
    function: 0x2e7de9,
    r#type: 0x007197,
    tag: 0xf52a65,
    attribute: 0x8c6c3e,
    operator: 0x006a83,
};
const TOKYO_NIGHT: RolePalette = RolePalette {
    base: 0xc0caf5,
    keyword: 0xbb9af7,
    comment: 0x565f89,
    string: 0x9ece6a,
    number: 0xff9e64,
    function: 0x7aa2f7,
    r#type: 0x2ac3de,
    tag: 0xf7768e,
    attribute: 0xe0af68,
    operator: 0x89ddff,
};

const ONE_LIGHT: RolePalette = RolePalette {
    base: 0x383a42,
    keyword: 0xa626a4,
    comment: 0xa0a1a7,
    string: 0x50a14f,
    number: 0x986801,
    function: 0x4078f2,
    r#type: 0xc18401,
    tag: 0xe45649,
    attribute: 0x986801,
    operator: 0x0184bc,
};
const ONE_DARK: RolePalette = RolePalette {
    base: 0xabb2bf,
    keyword: 0xc678dd,
    comment: 0x5c6370,
    string: 0x98c379,
    number: 0xd19a66,
    function: 0x61afef,
    r#type: 0xe5c07b,
    tag: 0xe06c75,
    attribute: 0xd19a66,
    operator: 0x56b6c2,
};

const NORD_LIGHT: RolePalette = RolePalette {
    base: 0x2e3440,
    keyword: 0x5e81ac,
    comment: 0x6b7280,
    string: 0x4f6f3a,
    number: 0x8f5e91,
    function: 0x2e6f8e,
    r#type: 0x3b7f87,
    tag: 0x5e81ac,
    attribute: 0x8f5e91,
    operator: 0x5e81ac,
};
const NORD_DARK: RolePalette = RolePalette {
    base: 0xd8dee9,
    keyword: 0x81a1c1,
    comment: 0x616e88,
    string: 0xa3be8c,
    number: 0xb48ead,
    function: 0x88c0d0,
    r#type: 0x8fbcbb,
    tag: 0x81a1c1,
    attribute: 0x8fbcbb,
    operator: 0x81a1c1,
};

const GRUVBOX_LIGHT: RolePalette = RolePalette {
    base: 0x3c3836,
    keyword: 0x9d0006,
    comment: 0x928374,
    string: 0x79740e,
    number: 0x8f3f71,
    function: 0x427b58,
    r#type: 0xb57614,
    tag: 0x076678,
    attribute: 0xb57614,
    operator: 0xaf3a03,
};
const GRUVBOX_DARK: RolePalette = RolePalette {
    base: 0xebdbb2,
    keyword: 0xfb4934,
    comment: 0x928374,
    string: 0xb8bb26,
    number: 0xd3869b,
    function: 0x8ec07c,
    r#type: 0xfabd2f,
    tag: 0x83a598,
    attribute: 0xfabd2f,
    operator: 0xfe8019,
};

const SOLARIZED_LIGHT: RolePalette = RolePalette {
    base: 0x657b83,
    keyword: 0x859900,
    comment: 0x93a1a1,
    string: 0x2aa198,
    number: 0xd33682,
    function: 0x268bd2,
    r#type: 0xb58900,
    tag: 0x268bd2,
    attribute: 0xb58900,
    operator: 0x859900,
};
const SOLARIZED_DARK: RolePalette = RolePalette {
    base: 0x839496,
    keyword: 0x859900,
    comment: 0x586e75,
    string: 0x2aa198,
    number: 0xd33682,
    function: 0x268bd2,
    r#type: 0xb58900,
    tag: 0x268bd2,
    attribute: 0xb58900,
    operator: 0x859900,
};
