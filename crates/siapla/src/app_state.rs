use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, watch};

#[derive(Clone, Debug)]
pub enum CalculationState {
    Modified,
    Calculating,
    Finished,
}

#[derive(Debug, Clone)]
pub struct AppState {
    /// broadcast channel for modification events (sender identity as String)
    pub modify_tx: broadcast::Sender<String>,
    /// manual recalculation trigger (single receiver expected)
    pub manual_tx: mpsc::UnboundedSender<()>,
    /// watch channel for current calculation state
    pub state_tx: watch::Sender<CalculationState>,
}

impl AppState {
    /// Create a new AppState and return it together with the manual receiver.
    pub fn new() -> (Arc<Self>, mpsc::UnboundedReceiver<()>) {
        let (modify_tx, _modify_rx) = broadcast::channel(16);
        let (manual_tx, manual_rx) = mpsc::unbounded_channel();
        let (state_tx, _state_rx) = watch::channel(CalculationState::Modified);
        (Arc::new(Self { modify_tx, manual_tx, state_tx }), manual_rx)
    }

    pub fn notify_modified(&self, sender: impl Into<String>) {
        let _ = self.modify_tx.send(sender.into());
    }

    pub fn trigger_manual(&self) {
        let _ = self.manual_tx.send(());
    }

    pub fn set_state(&self, state: CalculationState) {
        let _ = self.state_tx.send(state);
    }
}
