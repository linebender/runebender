//! The 'preview' tool
//!
//! This is generally represented as the 'hand', and allows the user to pan around
//! the workspace by clicking and dragging, although whether this makes sense
//! in the era of the touchpad is an open question.

use crate::tools::{Tool, ToolId};

/// The state of the preview tool.
#[derive(Debug, Default, Clone)]
pub struct Preview {}

impl Tool for Preview {
    fn name(&self) -> ToolId {
        "Preview"
    }
}
