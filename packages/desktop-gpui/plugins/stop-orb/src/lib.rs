//! `bachuc-plugin-stop-orb`: the native client of the stop-orb plugin
//! (`hiep-plugins/plugins/stop-orb/index.client.tsx`, ADR-0045). It replaces the composer's red
//! stop face with the `solving` orb of the `thinking-orbs` library, drawn as a GPUI animation
//! (`orb`, the pure frame rules) on the button (`face`).

mod face;
mod orb;

use crate::face::OrbFace;
use bachuc_plugin_host::{Contributions, NativePlugin, PluginContext, StopButton, StopButtonProps};
use gpui_kit::{AnyElement, App, AppContext as _, Entity, IntoElement as _};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const PLUGIN_ID: &str = "stop-orb";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(StopOrb)
}

struct StopOrb;

impl NativePlugin for StopOrb {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, _ctx: PluginContext, contributions: &mut Contributions, _cx: &mut App) {
        // One face per agent keeps its clock and hover state across the composer's renders. The
        // map lives in the closure, so deactivating the plugin drops the faces.
        let faces: Rc<RefCell<HashMap<String, Entity<OrbFace>>>> = Rc::default();
        contributions.add_stop_button(StopButton {
            id: "solving-orb",
            render: Rc::new(move |props, _window, cx| face_for(&faces, props, cx)),
        });
    }
}

fn face_for(
    faces: &RefCell<HashMap<String, Entity<OrbFace>>>,
    props: &StopButtonProps,
    cx: &mut App,
) -> AnyElement {
    let face = faces
        .borrow_mut()
        .entry(props.agent_id.clone())
        .or_insert_with(|| cx.new(|_| OrbFace::new(props)))
        .clone();
    face.update(cx, |face, cx| face.set_props(props, cx));
    face.into_any_element()
}
