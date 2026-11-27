//! A layout file in the five-region shape ADR 0055 wrote, read as the tree
//! it means: [`Layout::classic`] with each region's block filled and hidden
//! as the file says.
//!
//! ```json
//! { "toolbar": { "mod": "toolbar", "props": {} }, "sidebar": { "hidden": true } }
//! ```

use serde_json::{Map, Value};

use super::layout::{Layout, LayoutError, RegionNode};
use super::node::Content;
use super::{Overlay, Region};

/// The layout the region-shaped `fields` mean.
pub(super) fn read(mut fields: Map<String, Value>) -> Result<Layout, LayoutError> {
    let mut layout = Layout::classic();
    for region in Region::ALL {
        let Some(value) = fields.remove(region.name()) else {
            continue;
        };
        let node: RegionNode = serde_json::from_value(value)
            .map_err(|error| LayoutError(format!("`{}`: {error}", region.name())))?;
        if region == Region::Dock {
            *layout.overlay_mut(Overlay::Dock) = node;
            continue;
        }
        let Some(block) = layout.root.find_mut(region.name()) else {
            continue;
        };
        block.hidden = node.hidden;
        if let Some(name) = node.fill {
            block.content = Some(Content::Mod {
                name,
                props: node.props,
            });
        }
    }
    for overlay in Overlay::ALL {
        if let Some(value) = fields.remove(overlay.name()) {
            *layout.overlay_mut(overlay) = serde_json::from_value(value)
                .map_err(|error| LayoutError(format!("`{}`: {error}", overlay.name())))?;
        }
    }
    if let Some(key) = fields.keys().next() {
        return Err(LayoutError(format!(
            "unknown field `{key}`: a layout names a \"type\", or the regions {}",
            Region::ALL.map(Region::name).join(", ")
        )));
    }
    layout.title = None;
    Ok(layout)
}
