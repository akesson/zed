//! Orkesto fork: hooks that let `orkesto_zed` run local shell terminals on the Orkesto Host, so
//! they survive a restart of Zed (spikes/zed-reattach). Zed calls these, and `orkesto_zed`
//! registers them. With no hooks registered, Zed behaves exactly as upstream.

use crate::Terminal;
use gpui::{App, WeakEntity};
use collections::HashMap;
use std::{path::Path, sync::OnceLock};

/// Env var in `Project::create_terminal_shell_with_key`'s env: the key of a terminal to restore.
pub const RESTORE_KEY_ENV: &str = "ORKESTO_TERMINAL_KEY";

/// What runs instead of the shell.
pub struct Wrapped {
    pub key: u64,
    pub program: String,
    pub args: Vec<String>,
    /// Reattaching to a running shell: don't type activation scripts into it.
    pub restored: bool,
}

pub trait Hooks: Send + Sync + 'static {
    /// A local, interactive shell terminal is about to run `program args` in `cwd` with `env`.
    /// `env` may carry [`RESTORE_KEY_ENV`].
    fn wrap(
        &self,
        cwd: Option<&Path>,
        program: &str,
        args: &[String],
        env: &mut HashMap<String, String>,
    ) -> Option<Wrapped>;
    /// The foreground process group of the real shell behind `key` (for titles and cwd).
    fn foreground_pid(&self, key: u64) -> Option<u32>;
    /// Persist `key` for a terminal item; called on every workspace serialization.
    fn save(&self, workspace_id: i64, item_id: u64, key: u64, cx: &mut App);
    /// The key stored for a terminal item, if any.
    fn load(&self, workspace_id: i64, item_id: u64, cx: &mut App) -> Option<u64>;
    /// The terminal's item was removed from a pane: closed, or moved to another pane.
    fn removed(&self, key: u64, terminal: WeakEntity<Terminal>, cx: &mut App);
    /// Terminals outside panes (the agent panel's): persist `key` under a stable `name`.
    fn save_named(&self, _name: &str, _key: u64, _cx: &mut App) {}
    /// The key stored under `name`, if any.
    fn load_named(&self, _name: &str, _cx: &mut App) -> Option<u64> {
        None
    }
    /// A terminal outside panes was closed for good (no move possible): end its shell.
    fn closed(&self, _key: u64) {}
    /// Drop what's stored under `name` (its terminal was closed).
    fn forget_named(&self, _name: &str, _cx: &mut App) {}
}

static HOOKS: OnceLock<Box<dyn Hooks>> = OnceLock::new();

pub fn set_hooks(hooks: impl Hooks) {
    let _ = HOOKS.set(Box::new(hooks));
}

pub fn hooks() -> Option<&'static dyn Hooks> {
    HOOKS.get().map(|hooks| hooks.as_ref())
}
