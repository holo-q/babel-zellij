//! babel-zellij — Live agent session status plugin for zellij.
//!
//! Renders a status bar showing babel agent session state: working counts,
//! awaiting indicators, per-pane activity. Receives push updates from
//! `babel zellij-bridge` via zellij pipe IPC — no polling.
//!
//! ## Architecture
//!
//! ```text
//! babel daemon ──paint stream──→ babel zellij-bridge ──zellij pipe──→ this plugin
//!                                                                         │
//!                                                                    renders status
//! ```
//!
//! The bridge process subscribes to babel's SubscribePaint IPC and translates
//! PaintEvents into a compact JSON message sent via `zellij pipe --name babel`.
//! This plugin receives the message, updates its internal state, and re-renders.
//!
//! ## Usage
//!
//! In your zellij layout or config:
//! ```kdl
//! pane size=1 borderless=true {
//!     plugin location="file:/path/to/babel-zellij.wasm"
//! }
//! ```
//!
//! Then start the bridge: `babel zellij-bridge`

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

/// State pushed from `babel zellij-bridge` via pipe.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct BabelState {
    /// Formatted status: "●●○ 2 working | 1 await | 3 tracked"
    status: String,
    /// Raw counts
    working: u32,
    awaiting: u32,
    tracked: u32,
    /// Per-pane state, keyed by babel paint ID
    panes: BTreeMap<String, PaneState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PaneState {
    agent: String,
    state: String,
    title: String,
    color: String,
    indicator: String,
}

struct State {
    babel: BabelState,
    cols: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            babel: BabelState::default(),
            cols: 80,
        }
    }
}

register_plugin!(State);

const PIPE_NAME: &str = "babel";

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        // Subscribe to events we care about
        subscribe(&[
            EventType::ModeUpdate,      // Re-render on mode change
            EventType::PaneUpdate,      // Track pane changes for layout
        ]);

        // Request pipe messages named "babel"
        // The bridge sends state updates via: zellij pipe --name babel -- <json>
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

        self.babel.status = "babel: connecting...".to_string();
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PaneUpdate(_manifest) => {
                // Could use this to track zellij's own pane state
                // For now, we rely on babel's paint stream for agent state
                false
            }
            Event::ModeUpdate(_) => {
                // Re-render on mode change (status bar may need refresh)
                true
            }
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        // Receive state updates from babel zellij-bridge
        if pipe_message.name == PIPE_NAME {
            if let Some(payload) = &pipe_message.payload {
                if let Ok(state) = serde_json::from_str::<BabelState>(payload) {
                    self.babel = state;
                    return true; // re-render
                }
            }
        }
        false
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.cols = cols;

        if rows == 0 || cols == 0 {
            return;
        }

        // Single-row status bar mode (typical: pane size=1)
        if rows == 1 {
            self.render_status_bar(cols);
            return;
        }

        // Multi-row: status bar + per-pane details
        self.render_status_bar(cols);
        if rows > 1 {
            self.render_pane_list(rows - 1, cols);
        }
    }
}

impl State {
    fn render_status_bar(&self, cols: usize) {
        let b = &self.babel;

        if b.tracked == 0 {
            let msg = if b.status.is_empty() {
                "babel: no sessions"
            } else {
                &b.status
            };
            let padded = format!("{:width$}", msg, width = cols);
            print!("{}", &padded[..padded.len().min(cols)]);
            return;
        }

        // Build status segments
        let mut segments: Vec<Segment> = Vec::new();

        // Dots: one per tracked pane
        let dots: String = (0..b.tracked)
            .map(|i| if i < b.working { '●' } else { '○' })
            .collect();
        segments.push(Segment::new(&dots));

        // Counts
        if b.working > 0 {
            segments.push(Segment::colored(
                &format!(" {} working", b.working),
                Color::Green,
            ));
        }
        if b.awaiting > 0 {
            segments.push(Segment::colored(
                &format!(" {} await", b.awaiting),
                Color::Yellow,
            ));
        }
        segments.push(Segment::dim(&format!(" {} tracked", b.tracked)));

        // Per-pane mini indicators (if space permits)
        if !b.panes.is_empty() {
            segments.push(Segment::dim(" │ "));
            for (i, (_id, pane)) in b.panes.iter().enumerate() {
                if i > 0 {
                    segments.push(Segment::dim(" "));
                }
                let color = match pane.state.as_str() {
                    "working" => Color::Green,
                    "awaiting" => Color::Yellow,
                    "active" | "busy" => Color::Cyan,
                    _ => Color::Gray,
                };
                segments.push(Segment::colored(
                    &format!("{}{}", pane.indicator, pane.agent.chars().next().unwrap_or('?')),
                    color,
                ));
            }
        }

        // Render segments, padding to fill the row
        let mut output = String::new();
        let mut used = 0;
        for seg in &segments {
            if used + seg.text.len() > cols {
                break;
            }
            output.push_str(&seg.render());
            used += seg.text.len();
        }

        // Pad remaining space
        if used < cols {
            output.push_str(&" ".repeat(cols - used));
        }

        print!("{}", output);
    }

    fn render_pane_list(&self, rows: usize, cols: usize) {
        let b = &self.babel;

        for (i, (_id, pane)) in b.panes.iter().enumerate() {
            if i >= rows {
                break;
            }

            let color = match pane.state.as_str() {
                "working" => Color::Green,
                "awaiting" => Color::Yellow,
                "active" | "busy" => Color::Cyan,
                _ => Color::Gray,
            };

            let line = format!(
                " {} {} — {}",
                pane.indicator, pane.agent, pane.title
            );
            let padded = format!("{:width$}", line, width = cols);
            let seg = Segment::colored(&padded[..padded.len().min(cols)], color);
            println!("{}", seg.render());
        }

        // Fill remaining rows
        let pane_count = b.panes.len().min(rows);
        for _ in pane_count..rows {
            println!("{}", " ".repeat(cols));
        }
    }
}

// ─── Rendering helpers ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum Color {
    Green,
    Yellow,
    Cyan,
    Gray,
    Default,
}

struct Segment {
    text: String,
    color: Color,
    dim: bool,
}

impl Segment {
    fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            color: Color::Default,
            dim: false,
        }
    }

    fn colored(text: &str, color: Color) -> Self {
        Self {
            text: text.to_string(),
            color,
            dim: false,
        }
    }

    fn dim(text: &str) -> Self {
        Self {
            text: text.to_string(),
            color: Color::Default,
            dim: true,
        }
    }

    fn render(&self) -> String {
        let (fg_start, fg_end) = match self.color {
            Color::Green => ("\x1b[32m", "\x1b[0m"),
            Color::Yellow => ("\x1b[33m", "\x1b[0m"),
            Color::Cyan => ("\x1b[36m", "\x1b[0m"),
            Color::Gray => ("\x1b[90m", "\x1b[0m"),
            Color::Default if self.dim => ("\x1b[2m", "\x1b[0m"),
            Color::Default => ("", ""),
        };
        format!("{}{}{}", fg_start, self.text, fg_end)
    }
}
