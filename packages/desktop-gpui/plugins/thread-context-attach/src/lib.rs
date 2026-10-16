//! `bachuc-plugin-thread-context-attach`: the native client of the thread-context-attach plugin
//! (`hiep-plugins/plugins/thread-context-attach/shared/threads.ts`, ADR-0045). It adds the
//! `Thread` attachment source. The composer's picker searches through `thread-context.search`;
//! `thread-context.list-threads` and `thread-context.get-thread-snapshot` stay server-only.

use bachuc_icons::IconName;
use bachuc_plugin_host::{
    AttachmentPicker, AttachmentSource, Contributions, NativePlugin, PluginContext,
};
use gpui_kit::App;

const PLUGIN_ID: &str = "thread-context-attach";
const SEARCH_METHOD: &str = "thread-context.search";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(ThreadContextAttach)
}

struct ThreadContextAttach;

impl NativePlugin for ThreadContextAttach {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, _ctx: PluginContext, contributions: &mut Contributions, _cx: &mut App) {
        contributions.add_attachment_source(thread_source());
    }
}

/// `threadAttachments` of `shared/threads.ts`. The chip has no `on_open`: Electron adds none.
fn thread_source() -> AttachmentSource {
    AttachmentSource {
        id: "thread",
        title: "Thread".into(),
        icon: IconName::MessagesSquare,
        picker: Some(AttachmentPicker {
            title: "Attach another thread's last reply".into(),
            search_placeholder: "Search threads by title, provider, or folder".into(),
            search_method: SEARCH_METHOD,
        }),
        on_open: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_plugin_host::{ContributionIndex, decode_attachment_search};
    use serde_json::json;

    #[test]
    fn picker_uses_search_method() {
        let mut contributions = Contributions::default();
        contributions.add_attachment_source(thread_source());
        let index = ContributionIndex::build([(PLUGIN_ID, &contributions)]);
        let [source] = index.attachment_sources() else {
            panic!("expected one attachment source");
        };
        assert_eq!(source.plugin_id, PLUGIN_ID);
        assert_eq!(source.value.id, "thread");
        assert_eq!(source.value.title.as_ref(), "Thread");
        assert_eq!(source.value.icon, IconName::MessagesSquare);
        assert!(source.value.on_open.is_none());
        let picker = source.value.picker.as_ref().unwrap();
        assert_eq!(picker.search_method, "thread-context.search");
        assert_eq!(picker.title.as_ref(), "Attach another thread's last reply");
        assert_eq!(
            picker.search_placeholder.as_ref(),
            "Search threads by title, provider, or folder"
        );
    }

    #[test]
    fn search_payload_decodes() {
        let items = decode_attachment_search(json!({
            "items": [{
                "id": "agent-1",
                "identifier": "agent-1",
                "title": "Fix the build",
                "subtitle": "claude · /w/app",
                "url": "paseo-thread://agent-1",
                "text": "The last reply.",
                "resourceType": "thread",
            }],
        }))
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Fix the build");
        assert_eq!(items[0].subtitle.as_deref(), Some("claude · /w/app"));
        assert_eq!(items[0].resource_type, "thread");
        assert!(decode_attachment_search(json!({"threads": []})).is_err());
    }
}
