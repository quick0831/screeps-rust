use std::cmp::{Ordering, max, min};

use pathfinding::prelude::*;

pub struct CreepSizePlanner<'a> {
    pub current_sizes: &'a [u8],
    pub target_plan: &'a [u8],
    pub max_creep_count: u8,
}

impl CreepSizePlanner<'_> {
    pub fn solve(&self) -> Option<u8> {
        let Self {
            current_sizes,
            target_plan,
            max_creep_count,
        } = *self;

        if current_sizes.len() >= max_creep_count as usize {
            return None;
        }
        let last_slot = current_sizes.len() + 1 == max_creep_count as usize;

        let current_total_size: u16 = current_sizes.iter().map(|s| *s as u16).sum();
        let target_total_size: u16 = target_plan.iter().map(|s| *s as u16).sum();
        let needed_size = target_total_size.saturating_sub(current_total_size);
        let needed_size = min(needed_size, 50) as u8;

        if needed_size == 0 {
            return None;
        }

        let mut weights = Matrix::new(
            current_sizes.len() + 1,
            max(current_sizes.len() + 1, target_plan.len()),
            0,
        );

        for (i, s1) in current_sizes.iter().enumerate() {
            for (j, s2) in target_plan.iter().enumerate() {
                weights[(i, j)] = s1.abs_diff(*s2) as i8;
            }
        }

        if last_slot {
            for (j, s2) in target_plan.iter().enumerate() {
                weights[(current_sizes.len(), j)] = needed_size.abs_diff(*s2) as i8;
            }
        } else {
            for (j, s2) in target_plan.iter().enumerate() {
                weights[(current_sizes.len(), j)] = s2.saturating_sub(needed_size) as i8;
            }
        }

        let (_, matching) = kuhn_munkres_min(&weights);

        let matched_size = *target_plan.get(*matching.last()?)?;

        match matched_size.cmp(&needed_size) {
            Ordering::Less => {
                if needed_size == 1 {
                    None
                } else if last_slot {
                    Some(needed_size - 1)
                } else {
                    Some(matched_size)
                }
            }
            Ordering::Equal => Some(needed_size),
            Ordering::Greater => Some(needed_size + 1),
        }
    }
}
