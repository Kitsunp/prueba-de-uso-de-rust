use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{VnError, VnResult};
use crate::event::Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Script {
    pub events: Vec<Event>,
    pub labels: HashMap<String, usize>,
}

impl Script {
    pub fn new(events: Vec<Event>, labels: HashMap<String, usize>) -> Self {
        Self { events, labels }
    }

    pub fn from_json(input: &str) -> VnResult<Self> {
        serde_json::from_str(input).map_err(|err| {
            let (offset, length) = json_error_span(input, &err);
            VnError::Serialization {
                message: err.to_string(),
                src: input.to_string(),
                span: (offset, length).into(),
            }
        })
    }

    pub fn start_index(&self) -> VnResult<usize> {
        self.labels
            .get("start")
            .copied()
            .ok_or_else(|| VnError::InvalidScript("missing 'start' label".to_string()))
    }
}

fn json_error_span(input: &str, error: &serde_json::Error) -> (usize, usize) {
    let line = error.line();
    let column = error.column();
    if line == 0 || column == 0 {
        return (0, 1);
    }
    let mut current_line = 1usize;
    let mut offset = 0usize;
    for chunk in input.split_inclusive('\n') {
        if current_line == line {
            let column_index = column.saturating_sub(1);
            let byte_index = chunk
                .char_indices()
                .nth(column_index)
                .map(|(idx, _)| idx)
                .unwrap_or(chunk.len().saturating_sub(1));
            offset += byte_index;
            return (offset, 1);
        }
        offset += chunk.len();
        current_line += 1;
    }
    (input.len().saturating_sub(1), 1)
}
