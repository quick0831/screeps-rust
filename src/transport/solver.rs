use std::arch::wasm32::unreachable;
use std::cmp::max;
use std::cmp::min;

use num::Rational32;

use crate::transport::Priority;
use crate::transport::TransportAllocator;
use crate::transport::TransportTask;
use crate::transport::alloc::Info;
use crate::transport::alloc::TaskType;
use crate::transport::task::TransportAllocation;
use crate::utils::PriorityQueue;

pub fn solve(mut allocator: TransportAllocator) -> TransportAllocation {
    let mut alloc = vec![None; allocator.id_counter as usize];

    // Remove corresponded amount already queued
    for info in &allocator.creeps {
        let Some(task) = info.task else { continue };
        match task {
            TransportTask::Pickup(id) => {
                if let Some(export) = allocator.exports.iter_mut().find(|ex| ex.target.id() == id) {
                    export.amount = export.amount.saturating_sub(info.free_capacity);
                }
            }
            TransportTask::Supply(id) => {
                if let Some(import) = allocator.imports.iter_mut().find(|ex| ex.target.id() == id) {
                    import.amount = import.amount.saturating_sub(info.used_capacity);
                }
            }
        }
    }

    allocator.exports.retain(|info| info.amount > 0);
    allocator.imports.retain(|info| info.amount > 0);

    // exports
    let creep_export: u32 = allocator
        .creeps
        .iter()
        .filter(|info| matches!(info.task_type, TaskType::Export))
        // Remove corresponded amount already queued
        .filter(|info| info.task.is_none())
        .map(|info| info.used_capacity)
        .sum();
    let creep_bidir: u32 = allocator
        .creeps
        .iter()
        .filter(|info| matches!(info.task_type, TaskType::Bidir))
        .map(|info| info.used_capacity)
        .sum();
    let structure_export: u32 = allocator.exports.iter().map(|info| info.amount).sum();
    let structure_export_active: u32 = allocator
        .exports
        .iter()
        .filter(|info| info.priority != Priority::Passive)
        .map(|info| info.amount)
        .sum();

    // imports
    let creep_import: u32 = allocator
        .creeps
        .iter()
        .filter(|info| matches!(info.task_type, TaskType::Import))
        // Remove corresponded amount already queued
        .filter(|info| info.task.is_none())
        .map(|info| info.free_capacity)
        .sum();
    let structure_import: u32 = allocator.imports.iter().map(|info| info.amount).sum();
    let structure_import_active: u32 = allocator
        .imports
        .iter()
        .filter(|info| info.priority != Priority::Passive)
        .map(|info| info.amount)
        .sum();

    // sum of all network flow
    let total_export = creep_export + creep_bidir + structure_export;
    let active_export = creep_export + creep_bidir + structure_export_active;
    let total_import = creep_import + structure_import;
    let active_import = creep_import + structure_import_active;

    let network_cap = min(total_export, total_import);
    let mut export_requested = min(network_cap, max(structure_export_active, active_import));
    let mut import_requested = min(network_cap, max(active_export, structure_import_active));

    let infos: PriorityQueue<_, (usize, &Info)> = allocator
        .creeps
        .iter()
        .enumerate()
        .map(|(id, info)| {
            let single_sided = matches!(info.task_type, TaskType::Import | TaskType::Export);
            ((single_sided, info.size), (id, info))
        })
        .collect();

    // dispatch task for creeps without a task
    for (_, (id, info)) in infos {
        if info.task.is_some() || info.spawning {
            continue;
        }

        let can_import = matches!(info.task_type, TaskType::Bidir | TaskType::Export);
        let import_candidate = if can_import && import_requested > 0 {
            allocator
                .imports
                .iter()
                .enumerate()
                .filter_map(|(idx, import)| {
                    let amount = min(import.amount, info.used_capacity);
                    if amount != 0 {
                        let distance = max(1, import.pos.get_range_to(info.pos));
                        let score = Rational32::new(amount as i32, distance as i32);
                        Some((idx, (import.priority, score)))
                    } else {
                        None
                    }
                })
                .max_by_key(|(_, score)| *score)
        } else {
            None
        };

        let can_export = matches!(info.task_type, TaskType::Bidir | TaskType::Import);
        let export_candidate = if can_export && export_requested > 0 {
            allocator
                .exports
                .iter()
                .enumerate()
                .filter_map(|(idx, export)| {
                    let amount = min(export.amount, info.free_capacity);
                    if amount != 0 {
                        let distance = max(1, export.pos.get_range_to(info.pos));
                        let score = Rational32::new(amount as i32, distance as i32);
                        Some((idx, (export.priority, score)))
                    } else {
                        None
                    }
                })
                .max_by_key(|(_, score)| *score)
        } else {
            None
        };

        let task = match (import_candidate, export_candidate) {
            (Some((id, im)), ex) if ex.is_none_or(|(_, ex)| im > ex) => {
                let mut import = allocator.imports.swap_remove(id);
                let id = import.target.id();
                let amount = min(import.amount, info.used_capacity);
                import_requested -= amount;
                import.amount -= amount;
                if import.amount > 0 {
                    allocator.imports.push(import);
                }
                TransportTask::Supply(id)
            }
            (_, Some((id, _))) => {
                let mut export = allocator.exports.swap_remove(id);
                let id = export.target.id();
                let amount = min(export.amount, info.free_capacity);
                export_requested -= amount;
                export.amount -= amount;
                if export.amount > 0 {
                    allocator.exports.push(export);
                }
                TransportTask::Pickup(id)
            }
            (None, None) => continue,
            (Some(_), None) => unreachable(),
        };

        alloc[id] = Some(task);
    }

    TransportAllocation::new(alloc)
}
