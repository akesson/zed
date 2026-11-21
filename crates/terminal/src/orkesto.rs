//! Orkesto fork: hooks that let `orkesto_zed` run local shell terminals on the Orkesto Host, so
//! they survive a restart of Zed (spikes/zed-reattach). Zed calls these, and `orkesto_zed`
//! registers them. With no hooks registered, Zed behaves exactly as upstream.

use crate::Terminal;
use gpui::{App, WeakEntity};
use collections::HashMap;
use std::{path::Path, sync::OnceLock};

/// Env var in `Project::create_terminal_shell_restoring`'s env: a [`Restore`], encoded. The
/// terminal builder takes it out and hands it to [`Hooks::wrap`]; the shell never sees it.
pub const RESTORE_ENV: &str = "ORKESTO_TERMINAL_RESTORE";

/// Which Host shell a new terminal should show, if it still runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Restore {
    /// The shell with this key (a pane's terminal: the key was saved with its item).
    Key(u64),
    /// The shell whose key `orkesto_zed` stores under this name (an agent panel terminal thread).
    /// The hooks keep the name pointing at the shell the terminal ends up showing.
    Name(String),
}

impl Restore {
    pub fn encode(&self) -> String {
        match self {
            Restore::Key(key) => format!("key:{key}"),
            Restore::Name(name) => format!("name:{name}"),
        }
    }

    pub fn decode(value: &str) -> Option<Self> {
        if let Some(key) = value.strip_prefix("key:") {
            return key.parse().ok().map(Restore::Key);
        }
        value.strip_prefix("name:").map(|name| Restore::Name(name.to_string()))
    }
}

/// What runs instead of the shell.
pub struct Wrapped {
    pub key: u64,
    pub program: String,
    pub args: Vec<String>,
    /// Reattaching to a running shell: don't type activation scripts into it.
    pub restored: bool,
}

pub trait Hooks: Send + Sync + 'static {
    /// A local, interactive shell terminal is about to run `program args` in `cwd` with `env`,
    /// showing the shell `restore` names if it still runs.
    fn wrap(
        &self,
        cwd: Option<&Path>,
        program: &str,
        args: &[String],
        restore: Option<Restore>,
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

