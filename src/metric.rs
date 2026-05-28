use std::collections::VecDeque;

use serde::Deserialize;
use serde::Serialize;

use crate::utils::fir_low_pass;

const FILTER_LEN: usize = 63;
const FILTER: [f64; FILTER_LEN] = fir_low_pass(0.1);

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Metric {
    accumulator: i32,
    buffer: VecDeque<i32>,
}

impl Metric {
    pub fn record_add(&mut self, amount: i32) {
        self.accumulator += amount;
    }

    pub fn record_finish(&mut self) {
        if self.buffer.len() != FILTER_LEN {
            self.buffer.clear();
            for _ in 0..FILTER_LEN {
                self.buffer.push_back(0);
            }
        }
        self.buffer.pop_front();
        self.buffer.push_back(self.accumulator);
        self.accumulator = 0;
    }

    pub fn calculate_output(&self) -> f64 {
        self.buffer
            .iter()
            .cloned()
            .zip(FILTER.iter().cloned())
            .map(|(a, b)| a as f64 * b)
            .sum()
    }
}
