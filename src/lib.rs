pub mod time;
pub mod parse;
pub mod state;
pub mod alert;
pub mod rules;
pub mod config;
pub mod scrape;
pub mod notify;
pub mod rpc;

use crate::notify::{format_message, Notifier};
use crate::parse::Snapshot;
use crate::rules::Engine;
use crate::state::State;

/// One evaluation cycle: evaluate rules and send transitions. Returns the number of sends.
pub fn run_once(
    engine: &mut Engine,
    state: &State,
    snap: &Snapshot,
    now_ms: i64,
    notifier: &dyn Notifier,
) -> usize {
    let mut sent = 0;
    for fired in engine.evaluate(state, snap, now_ms) {
        let text = format_message(&fired);
        match notifier.send(&text) {
            Ok(()) => sent += 1,
            Err(e) => eprintln!("notify error: {e:#}"),
        }
    }
    sent
}
