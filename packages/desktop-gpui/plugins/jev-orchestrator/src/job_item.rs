//! The `job` timeline renderer (`JobTimelineItem` of `index.client.tsx`): "id · status" over the
//! message. The server appends the item in `server/paseo.ts`.

use bachuc_plugin_host::TimelineRenderInput;
use bachuc_theme::metrics::{FONT_BASE, SPACE_3, WEIGHT_SEMIBOLD};
use bachuc_theme::{tokens, ui_font_size, zoomed};
use gpui_kit::*;
use serde::Deserialize;
use serde_json::Value;

const UNAVAILABLE: &str = "Plugin timeline item unavailable.";
/// `jobItemStyle.gap`.
const GAP: f32 = 6.;

/// The renderer's schema. `children` is only checked, as the zod schema does.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct JobItemData {
    pub id: String,
    pub status: String,
    pub message: String,
    #[allow(dead_code)]
    pub children: Vec<String>,
}

impl JobItemData {
    /// `None` when the data does not fit the schema.
    pub(crate) fn decode(data: &Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }

    pub(crate) fn title(&self) -> String {
        format!("{} · {}", self.id, self.status)
    }
}

pub(crate) fn build(input: TimelineRenderInput, cx: &mut App) -> AnyView {
    let data = JobItemData::decode(&input.item.data);
    cx.new(|_| JobItem { data }).into()
}

struct JobItem {
    data: Option<JobItemData>,
}

impl Render for JobItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let size = ui_font_size(FONT_BASE, cx);
        let Some(data) = &self.data else {
            return div()
                .id("jev-job-unavailable")
                .role(Role::Note)
                .aria_label(UNAVAILABLE)
                .p(zoomed(SPACE_3, cx))
                .text_size(size)
                .text_color(colors.foreground_muted)
                .child(UNAVAILABLE)
                .into_any_element();
        };
        let title = data.title();
        div()
            .id("jev-job-item")
            .role(Role::Group)
            .aria_label(title.clone())
            .flex()
            .flex_col()
            .gap(zoomed(GAP, cx))
            .p(zoomed(SPACE_3, cx))
            .text_size(size)
            .line_height(relative(1.2))
            .child(
                div()
                    .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                    .text_color(colors.foreground)
                    .child(title),
            )
            .child(
                div()
                    .text_color(colors.foreground_muted)
                    .child(data.message.clone()),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::JobItemData;
    use serde_json::json;

    #[test]
    fn a_job_item_reads_like_the_server_appends_it() {
        let data = JobItemData::decode(&json!({
            "id": "t1", "status": "passed", "message": "Done", "children": ["c1", "c2"]
        }))
        .unwrap();
        assert_eq!(data.title(), "t1 · passed");
        assert_eq!(data.message, "Done");
    }

    #[test]
    fn data_that_misses_the_schema_is_rejected() {
        for data in [
            json!({"id": "t1", "status": "passed", "message": "Done"}),
            json!({"id": "t1", "status": 4, "message": "Done", "children": []}),
            json!({"id": "t1", "status": "passed", "message": "Done", "children": [1]}),
            json!(null),
        ] {
            assert_eq!(JobItemData::decode(&data), None, "{data}");
        }
    }
}
