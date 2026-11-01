//! Lane layout for the commit graph: which column each commit sits in and which lines connect
//! it to its neighbours, computed from `git log --topo-order` (children before parents).

use crate::git::CommitInfo;

/// What to draw for one commit row. Lines run between the row's top edge, its dot, and its
/// bottom edge; lanes are column indexes counted from the left.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Row {
    /// Column of this commit's dot.
    pub lane: usize,
    /// Lanes whose line arrives from above and ends at the dot.
    pub incoming: Vec<usize>,
    /// Lanes that run straight through the row without touching the dot.
    pub passing: Vec<usize>,
    /// Lanes the dot's line leaves toward at the bottom (one per parent).
    pub outgoing: Vec<usize>,
    /// Number of columns needed to draw this row.
    pub width: usize,
}

pub fn layout(commits: &[CommitInfo]) -> Vec<Row> {
    // For every lane, the commit hash it is waiting to meet (its next ancestor).
    let mut active: Vec<Option<String>> = Vec::new();
    let mut rows = Vec::with_capacity(commits.len());

    for commit in commits {
        let waiting: Vec<usize> = active
            .iter()
            .enumerate()
            .filter(|(_, hash)| hash.as_deref() == Some(commit.hash.as_str()))
            .map(|(lane, _)| lane)
            .collect();
        let lane = waiting
            .first()
            .copied()
            .unwrap_or_else(|| free_lane(&mut active));
        let passing: Vec<usize> = active
            .iter()
            .enumerate()
            .filter(|(index, hash)| hash.is_some() && !waiting.contains(index))
            .map(|(index, _)| index)
            .collect();
        for index in &waiting {
            active[*index] = None;
        }

        let mut outgoing = Vec::new();
        for (position, parent) in commit.parents.iter().enumerate() {
            let existing = active
                .iter()
                .position(|hash| hash.as_deref() == Some(parent.as_str()));
            let target = match existing {
                Some(index) => index,
                None if position == 0 => {
                    if lane >= active.len() {
                        active.resize(lane + 1, None);
                    }
                    lane
                }
                None => free_lane(&mut active),
            };
            active[target] = Some(parent.clone());
            outgoing.push(target);
        }
        while active.last().is_some_and(Option::is_none) {
            active.pop();
        }

        let width = [lane + 1, active.len()]
            .into_iter()
            .chain(waiting.iter().map(|l| l + 1))
            .chain(passing.iter().map(|l| l + 1))
            .chain(outgoing.iter().map(|l| l + 1))
            .max()
            .unwrap_or(1);
        rows.push(Row {
            lane,
            incoming: waiting,
            passing,
            outgoing,
            width,
        });
    }
    rows
}

/// The first empty lane, adding one when all are busy.
fn free_lane(active: &mut Vec<Option<String>>) -> usize {
    if let Some(index) = active.iter().position(Option::is_none) {
        index
    } else {
        active.push(None);
        active.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(hash: &str, parents: &[&str]) -> CommitInfo {
        CommitInfo {
            hash: hash.into(),
            parents: parents.iter().map(|p| p.to_string()).collect(),
            author: "A".into(),
            email: "a@x".into(),
            time: 0,
            refs: Vec::new(),
            subject: hash.into(),
        }
    }

    #[test]
    fn a_straight_history_stays_in_one_lane() {
        let rows = layout(&[commit("c", &["b"]), commit("b", &["a"]), commit("a", &[])]);
        assert!(rows.iter().all(|row| row.lane == 0 && row.width == 1));
        assert!(rows[0].incoming.is_empty(), "the tip has nothing above it");
        assert_eq!(rows[1].incoming, vec![0]);
        assert_eq!(rows[0].outgoing, vec![0]);
        assert!(rows[2].outgoing.is_empty(), "the root has no parent line");
    }

    #[test]
    fn a_merge_opens_a_second_lane_and_closes_it_again() {
        // M merges feature F into main; both descend from base B.
        let rows = layout(&[
            commit("M", &["A", "F"]),
            commit("A", &["B"]),
            commit("F", &["B"]),
            commit("B", &[]),
        ]);
        assert_eq!(rows[0].outgoing, vec![0, 1], "two parents, two lines");
        assert_eq!(rows[1].lane, 0);
        assert_eq!(rows[1].passing, vec![1], "feature lane runs past A");
        assert_eq!(rows[2].lane, 1);
        assert_eq!(rows[2].passing, vec![0]);
        // F's parent B is already awaited in lane 0, so the line bends back into it.
        assert_eq!(rows[2].outgoing, vec![0]);
        assert_eq!(rows[3].lane, 0);
        assert_eq!(rows[3].width, 1, "only one lane remains by the base");
    }

    #[test]
    fn separate_branch_tips_get_separate_lanes() {
        let rows = layout(&[commit("x", &["r"]), commit("y", &["r"]), commit("r", &[])]);
        assert_eq!(rows[0].lane, 0);
        assert_eq!(rows[1].lane, 1);
        assert_eq!(rows[1].outgoing, vec![0], "y bends into r's lane");
        assert_eq!(rows[2].lane, 0);
    }
}
