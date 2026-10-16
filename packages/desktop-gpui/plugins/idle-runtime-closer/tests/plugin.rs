//! The crate's one public item. The rules have unit tests next to their code, because the
//! crate exports nothing else (ADR-0045).

#[test]
fn the_plugin_id_is_idle_runtime_closer() {
    assert_eq!(
        bachuc_plugin_idle_runtime_closer::plugin().id(),
        "idle-runtime-closer"
    );
}
