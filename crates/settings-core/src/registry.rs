// Original Windhawk mod by caliberda
// Ported to standalone Rust by Antigravity

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StyleGroup {
    StacksAndFlow,
    ITabStages,
    SpatialAndShapes,
    PanelsAndGrids,
    ClassicAndSystem,
}

impl StyleGroup {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::StacksAndFlow => "3D Stacks & Flow",
            Self::ITabStages => "iTab Stages",
            Self::SpatialAndShapes => "Spatial & 3D Shapes",
            Self::PanelsAndGrids => "Panels, Grids & Lists",
            Self::ClassicAndSystem => "Classic & System",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleInfo {
    pub id: &'static str,
    pub display_name: &'static str,
    pub group: StyleGroup,
    pub description: &'static str,
    pub supported: bool,
}

pub const ALL_STYLES: &[StyleInfo] = &[
    // Group 1: 3D Stacks & Flow
    StyleInfo {
        id: "flip3d",
        display_name: "Flip 3D (Vista stack)",
        group: StyleGroup::StacksAndFlow,
        description: "Iconic cascading 3D isometric stack",
        supported: true,
    },
    StyleInfo {
        id: "cascade",
        display_name: "Cascade",
        group: StyleGroup::StacksAndFlow,
        description: "Diagonal cascade offset layout",
        supported: true,
    },
    StyleInfo {
        id: "coverFlow",
        display_name: "Cover Flow",
        group: StyleGroup::StacksAndFlow,
        description: "Reflective album art flow style",
        supported: true,
    },
    StyleInfo {
        id: "carousel",
        display_name: "Carousel",
        group: StyleGroup::StacksAndFlow,
        description: "Curved revolving card carousel",
        supported: true,
    },
    StyleInfo {
        id: "shuffle",
        display_name: "Shuffle",
        group: StyleGroup::StacksAndFlow,
        description: "Deck shuffling flip animation",
        supported: true,
    },
    StyleInfo {
        id: "domino",
        display_name: "Domino",
        group: StyleGroup::StacksAndFlow,
        description: "Falling dominoes cascading rotation",
        supported: true,
    },
    // Group 2: iTab Stages
    StyleInfo {
        id: "itab",
        display_name: "iTab (Black floor, app name)",
        group: StyleGroup::ITabStages,
        description: "Solid black reflective stage with dual labels",
        supported: true,
    },
    StyleInfo {
        id: "itabTitle",
        display_name: "iTab (Black floor, title only)",
        group: StyleGroup::ITabStages,
        description: "Solid black reflective stage with document title only",
        supported: true,
    },
    StyleInfo {
        id: "itabBackground",
        display_name: "iTab (Wallpaper floor, app name)",
        group: StyleGroup::ITabStages,
        description: "Transparent reflective stage blending into wallpaper",
        supported: true,
    },
    StyleInfo {
        id: "itabBackgroundTitle",
        display_name: "iTab (Wallpaper floor, title only)",
        group: StyleGroup::ITabStages,
        description: "Transparent reflective stage with document title only",
        supported: true,
    },
    // Group 3: Spatial & 3D Shapes
    StyleInfo {
        id: "helix",
        display_name: "Helix",
        group: StyleGroup::SpatialAndShapes,
        description: "Double-helical spiral arrangement",
        supported: true,
    },
    StyleInfo {
        id: "fan",
        display_name: "Fan",
        group: StyleGroup::SpatialAndShapes,
        description: "Radial fan-out deck presentation",
        supported: true,
    },
    StyleInfo {
        id: "panorama",
        display_name: "Panorama",
        group: StyleGroup::SpatialAndShapes,
        description: "Horizontal panoramic cylindrical arc",
        supported: true,
    },
    StyleInfo {
        id: "tunnel",
        display_name: "Tunnel",
        group: StyleGroup::SpatialAndShapes,
        description: "Deep perspective vortex tunnel",
        supported: true,
    },
    StyleInfo {
        id: "rolodex",
        display_name: "Rolodex",
        group: StyleGroup::SpatialAndShapes,
        description: "Vertical cylindrical desktop card index",
        supported: true,
    },
    StyleInfo {
        id: "cube",
        display_name: "Cube",
        group: StyleGroup::SpatialAndShapes,
        description: "Rotating 3D cubic surfaces",
        supported: true,
    },
    StyleInfo {
        id: "sphere",
        display_name: "Sphere",
        group: StyleGroup::SpatialAndShapes,
        description: "Spherical orbit arrangement",
        supported: true,
    },
    // Group 4: Panels, Grids & Lists
    StyleInfo {
        id: "grid",
        display_name: "Grid (Mission Control)",
        group: StyleGroup::PanelsAndGrids,
        description: "Non-overlapping tiled window matrix",
        supported: true,
    },
    StyleInfo {
        id: "windows11",
        display_name: "Windows 11",
        group: StyleGroup::PanelsAndGrids,
        description: "Modern centered card grid with subtle elevation",
        supported: true,
    },
    StyleInfo {
        id: "thumbnails",
        display_name: "Thumbnails",
        group: StyleGroup::PanelsAndGrids,
        description: "Horizontal row of proportional task previews",
        supported: true,
    },
    StyleInfo {
        id: "icons",
        display_name: "Icons and titles",
        group: StyleGroup::PanelsAndGrids,
        description: "High-density icon row with window labels",
        supported: true,
    },
    StyleInfo {
        id: "list",
        display_name: "Vertical list",
        group: StyleGroup::PanelsAndGrids,
        description: "Clean vertical task item list",
        supported: true,
    },
    // Group 5: Classic & System
    StyleInfo {
        id: "classic",
        display_name: "Classic (Windows XP)",
        group: StyleGroup::ClassicAndSystem,
        description: "Vintage Windows 2000 / XP 2D grid",
        supported: true,
    },
    StyleInfo {
        id: "native",
        display_name: "Native Windows Alt+Tab",
        group: StyleGroup::ClassicAndSystem,
        description: "Hands-off bypass to default Windows DWM switcher",
        supported: true,
    },
];

pub fn find_style(id_or_name: &str) -> Option<&'static StyleInfo> {
    let lower = id_or_name.to_ascii_lowercase();
    ALL_STYLES.iter().find(|s| {
        s.id.eq_ignore_ascii_case(&lower)
            || s.display_name.eq_ignore_ascii_case(&lower)
            || (lower == "iconrow" && s.id == "icons")
    })
}

pub fn get_styles_by_group(group: StyleGroup) -> Vec<&'static StyleInfo> {
    ALL_STYLES.iter().filter(|s| s.group == group).collect()
}
