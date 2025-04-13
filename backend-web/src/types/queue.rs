use crate::types::{TeamId, WorkItem};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Queue {
    next_team: Option<TeamId>,
}

impl Queue {
    pub fn new() -> Self {
        Self { next_team: None }
    }

    pub fn poll_task(&mut self, tasks: Vec<WorkItem>) -> Option<WorkItem> {
        let (tasks, next_team) = self.reorder_queue_step(tasks);
        self.next_team = next_team;
        tasks.into_iter().next()
    }

    pub fn reorder_queue(&self, tasks: Vec<WorkItem>) -> Vec<WorkItem> {
        self.reorder_queue_step(tasks).0
    }

    fn reorder_queue_step(&self, queue: Vec<WorkItem>) -> (Vec<WorkItem>, Option<TeamId>) {
        if queue.is_empty() {
            return (queue, None);
        }

        let mut result = Vec::new();
        let mut by_team: HashMap<TeamId, Vec<WorkItem>> =
            queue.into_iter().fold(HashMap::new(), |mut acc, next| {
                acc.entry(next.team.clone()).or_default().push(next);
                acc
            });

        // Test the newest first
        for tasks in by_team.values_mut() {
            tasks.sort_by(|a, b| b.insert_time.cmp(&a.insert_time));
        }

        let mut all_teams = by_team.keys().cloned().collect::<Vec<_>>();
        all_teams.sort();
        let start_index = match self.next_team {
            Some(ref team) => all_teams.iter().position(|t| t == team),
            None => Some(0),
        };
        let start_index = start_index.unwrap_or(0);

        for team in all_teams.iter().cycle().skip(start_index) {
            if by_team.is_empty() {
                break;
            }

            let Some(tasks) = by_team.get_mut(team) else {
                continue;
            };
            if let Some(work) = tasks.pop() {
                result.push(work);
            }

            if tasks.is_empty() {
                by_team.remove(team);
            }
        }

        (
            result,
            all_teams.get((start_index + 1) % all_teams.len()).cloned(),
        )
    }
}
