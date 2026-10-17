//! The crate's one public item. The rules have unit tests next to their code, because the
//! crate exports nothing else (ADR-0045).

#[test]
fn the_plugin_id_is_board() {
    assert_eq!(bachuc_plugin_board::plugin().id(), "board");
}
