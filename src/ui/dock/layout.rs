//! The dock layout model: a tree of splits whose leaves are stacks of tabbed panels.
//!
//! This module is plain data with no UI; [`super::view`] turns it into entities.

use bevy::{platform::collections::HashMap, prelude::*};

/// Identifies a node (split or leaf) in a [`DockLayout`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

/// Identifies a panel; an index into the panel registry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelId(pub u32);

/// Direction in which a split lays out its children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    /// Children side by side, left to right.
    #[default]
    Horizontal,
    /// Children stacked, top to bottom.
    Vertical,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DockNode {
    /// Children with relative sizes (weights; they need not sum to 1).
    Split {
        axis: Axis,
        children: Vec<NodeId>,
        sizes: Vec<f32>,
    },
    /// A stack of panels shown as tabs; one is visible at a time.
    Leaf { tabs: Vec<PanelId>, active: usize },
}

/// Where a dragged panel goes relative to the leaf it is dropped on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DropZone {
    /// Add it as a tab of the leaf.
    #[default]
    Center,
    /// Insert before this tab index, measured before removing the dragged panel.
    Tab(usize),
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct DockLayout {
    nodes: HashMap<NodeId, DockNode>,
    root: NodeId,
    next_id: u32,
}

impl Default for DockLayout {
    fn default() -> Self {
        let mut layout = Self {
            nodes: HashMap::default(),
            root: NodeId(0),
            next_id: 0,
        };
        layout.root = layout.add_leaf(Vec::new());
        layout
    }
}

impl DockLayout {
    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> &DockNode {
        &self.nodes[&id]
    }

    /// Adds a detached leaf; attach it with [`DockLayout::add_split`] or [`DockLayout::set_root`].
    pub fn add_leaf(&mut self, tabs: Vec<PanelId>) -> NodeId {
        self.insert_node(DockNode::Leaf { tabs, active: 0 })
    }

    /// Adds a detached split of the given `(child, size)` pairs.
    pub fn add_split(&mut self, axis: Axis, children: Vec<(NodeId, f32)>) -> NodeId {
        let (children, sizes) = children.into_iter().unzip();
        self.insert_node(DockNode::Split {
            axis,
            children,
            sizes,
        })
    }

    pub fn set_root(&mut self, root: NodeId) {
        self.root = root;
        self.prune();
    }

    /// The leaf that holds `panel`, if it is docked.
    pub fn leaf_of(&self, panel: PanelId) -> Option<NodeId> {
        self.nodes.iter().find_map(|(id, node)| match node {
            DockNode::Leaf { tabs, .. } if tabs.contains(&panel) => Some(*id),
            _ => None,
        })
    }

    /// Index of the tab currently shown in `leaf`.
    pub fn active_index(&self, leaf: NodeId) -> Option<usize> {
        match self.nodes.get(&leaf)? {
            DockNode::Leaf { active, .. } => Some(*active),
            DockNode::Split { .. } => None,
        }
    }

    /// The panel currently shown in `leaf`.
    pub fn active_panel(&self, leaf: NodeId) -> Option<PanelId> {
        match self.nodes.get(&leaf)? {
            DockNode::Leaf { tabs, active } => tabs.get(*active).copied(),
            DockNode::Split { .. } => None,
        }
    }

    /// Shows the tab at `index` in `leaf`.
    pub fn activate(&mut self, leaf: NodeId, index: usize) {
        if let Some(DockNode::Leaf { tabs, active }) = self.nodes.get_mut(&leaf)
            && index < tabs.len()
        {
            *active = index;
        }
    }

    /// Mutable access to a split's sizes, for resizing.
    pub fn split_sizes_mut(&mut self, split: NodeId) -> Option<&mut Vec<f32>> {
        match self.nodes.get_mut(&split)? {
            DockNode::Split { sizes, .. } => Some(sizes),
            DockNode::Leaf { .. } => None,
        }
    }

    /// Moves `panel` next to or into `target` (a leaf). Returns whether the layout changed.
    pub fn move_panel(&mut self, panel: PanelId, target: NodeId, zone: DropZone) -> bool {
        let Some(source) = self.leaf_of(panel) else {
            return false;
        };
        if !matches!(self.nodes.get(&target), Some(DockNode::Leaf { .. })) {
            return false;
        }
        if source == target {
            let Some(DockNode::Leaf { tabs, active }) = self.nodes.get_mut(&source) else {
                unreachable!()
            };
            let from = tabs.iter().position(|p| *p == panel).unwrap();
            if let DropZone::Tab(slot) = zone {
                let to = slot.min(tabs.len()) - usize::from(from < slot);
                if from == to {
                    return false;
                }
                tabs.remove(from);
                tabs.insert(to, panel);
                *active = to;
                return true;
            }
            // Dropping a panel on its own leaf: show it, or split it off from its siblings.
            if zone == DropZone::Center || tabs.len() == 1 {
                let changed = *active != from;
                *active = from;
                return changed;
            }
        }

        self.remove_panel(panel);
        match zone {
            DropZone::Center | DropZone::Tab(_) => {
                let DockNode::Leaf { tabs, active } = self.nodes.get_mut(&target).unwrap() else {
                    unreachable!()
                };
                *active = match zone {
                    DropZone::Tab(slot) => slot.min(tabs.len()),
                    _ => tabs.len(),
                };
                tabs.insert(*active, panel);
            }
            _ => self.split_leaf(target, panel, zone),
        }
        true
    }

    /// Removes `panel` from its leaf, removing the leaf (and collapsing splits) if it empties.
    pub fn remove_panel(&mut self, panel: PanelId) {
        let Some(leaf) = self.leaf_of(panel) else {
            return;
        };
        let DockNode::Leaf { tabs, active } = self.nodes.get_mut(&leaf).unwrap() else {
            unreachable!()
        };
        let from = tabs.iter().position(|p| *p == panel).unwrap();
        tabs.remove(from);
        if from < *active {
            *active -= 1;
        }
        *active = (*active).min(tabs.len().saturating_sub(1));
        if tabs.is_empty() && leaf != self.root {
            self.detach(leaf);
            self.nodes.remove(&leaf);
        }
    }

    fn insert_node(&mut self, node: DockNode) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        self.nodes.insert(id, node);
        id
    }

    fn parent_of(&self, child: NodeId) -> Option<(NodeId, usize)> {
        self.nodes.iter().find_map(|(id, node)| match node {
            DockNode::Split { children, .. } => children
                .iter()
                .position(|c| *c == child)
                .map(|index| (*id, index)),
            DockNode::Leaf { .. } => None,
        })
    }

    /// Unlinks `node` from its parent split. A split left with one child is replaced by it.
    fn detach(&mut self, node: NodeId) {
        let Some((parent, index)) = self.parent_of(node) else {
            return;
        };
        let DockNode::Split {
            children, sizes, ..
        } = self.nodes.get_mut(&parent).unwrap()
        else {
            unreachable!()
        };
        children.remove(index);
        sizes.remove(index);
        if children.len() == 1 {
            let only = children[0];
            self.replace(parent, only);
            self.nodes.remove(&parent);
        }
    }

    /// Puts `new` where `old` is in the tree (in its parent, or as the root).
    fn replace(&mut self, old: NodeId, new: NodeId) {
        match self.parent_of(old) {
            Some((parent, index)) => {
                if let DockNode::Split { children, .. } = self.nodes.get_mut(&parent).unwrap() {
                    children[index] = new;
                }
            }
            None => self.root = new,
        }
    }

    /// Puts a new leaf holding `panel` on the `zone` side of `target`.
    fn split_leaf(&mut self, target: NodeId, panel: PanelId, zone: DropZone) {
        let (axis, before) = match zone {
            DropZone::Left => (Axis::Horizontal, true),
            DropZone::Right => (Axis::Horizontal, false),
            DropZone::Top => (Axis::Vertical, true),
            DropZone::Bottom => (Axis::Vertical, false),
            DropZone::Center | DropZone::Tab(_) => unreachable!(),
        };
        let new_leaf = self.add_leaf(vec![panel]);

        // If the target already sits in a split along the same axis, become its sibling and
        // share its space; otherwise wrap the target in a new split.
        if let Some((parent, index)) = self.parent_of(target)
            && let DockNode::Split {
                axis: parent_axis,
                children,
                sizes,
            } = self.nodes.get_mut(&parent).unwrap()
            && *parent_axis == axis
        {
            let half = sizes[index] / 2.0;
            sizes[index] = half;
            let at = if before { index } else { index + 1 };
            children.insert(at, new_leaf);
            sizes.insert(at, half);
            return;
        }

        let children = if before {
            vec![(new_leaf, 1.0), (target, 1.0)]
        } else {
            vec![(target, 1.0), (new_leaf, 1.0)]
        };
        // Look up where `target` hangs before the new split also lists it as a child.
        let old_parent = self.parent_of(target);
        let split = self.add_split(axis, children);
        match old_parent {
            Some((parent, index)) => {
                if let DockNode::Split { children, .. } = self.nodes.get_mut(&parent).unwrap() {
                    children[index] = split;
                }
            }
            None => self.root = split,
        }
    }

    /// Drops nodes that are no longer reachable from the root.
    fn prune(&mut self) {
        let mut reachable = Vec::new();
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            reachable.push(id);
            if let Some(DockNode::Split { children, .. }) = self.nodes.get(&id) {
                stack.extend(children.iter().copied());
            }
        }
        self.nodes.retain(|id, _| reachable.contains(id));
    }

    /// Every leaf reachable from the root, in depth-first order.
    #[cfg(test)]
    fn leaves(&self) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            match &self.nodes[&id] {
                DockNode::Leaf { .. } => out.push(id),
                DockNode::Split { children, .. } => stack.extend(children.iter().rev().copied()),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: PanelId = PanelId(0);
    const B: PanelId = PanelId(1);
    const C: PanelId = PanelId(2);

    /// Root split: [leaf(A, B) | leaf(C)].
    fn two_leaves() -> (DockLayout, NodeId, NodeId) {
        let mut layout = DockLayout::default();
        let left = layout.add_leaf(vec![A, B]);
        let right = layout.add_leaf(vec![C]);
        let root = layout.add_split(Axis::Horizontal, vec![(left, 1.0), (right, 1.0)]);
        layout.set_root(root);
        (layout, left, right)
    }

    fn tabs(layout: &DockLayout, leaf: NodeId) -> Vec<PanelId> {
        match layout.node(leaf) {
            DockNode::Leaf { tabs, .. } => tabs.clone(),
            DockNode::Split { .. } => panic!("not a leaf"),
        }
    }

    #[test]
    fn tab_bar_drops_insert_at_the_indicated_gap() {
        let (mut layout, left, _) = two_leaves();
        assert!(layout.move_panel(C, left, DropZone::Tab(1)));
        assert_eq!(layout.root(), left);
        assert_eq!(tabs(&layout, left), vec![A, C, B]);
        assert_eq!(layout.active_panel(left), Some(C));
        assert!(layout.move_panel(A, left, DropZone::Tab(3)));
        assert_eq!(tabs(&layout, left), vec![C, B, A]);
        assert_eq!(layout.active_panel(left), Some(A));
        assert!(layout.move_panel(A, left, DropZone::Tab(0)));
        assert_eq!(tabs(&layout, left), vec![A, C, B]);
        let unchanged = layout.clone();
        for slot in [1, 2] {
            assert!(!layout.move_panel(C, left, DropZone::Tab(slot)));
            assert_eq!(layout, unchanged);
        }
    }

    #[test]
    fn move_to_center_adds_a_tab_and_activates_it() {
        let (mut layout, left, right) = two_leaves();
        assert!(layout.move_panel(B, right, DropZone::Center));
        assert_eq!(tabs(&layout, left), vec![A]);
        assert_eq!(tabs(&layout, right), vec![C, B]);
        assert_eq!(layout.active_panel(right), Some(B));
    }

    #[test]
    fn emptied_leaf_is_removed_and_split_collapses() {
        let (mut layout, _left, right) = two_leaves();
        layout.move_panel(C, layout.leaf_of(A).unwrap(), DropZone::Center);
        assert!(!layout.nodes.contains_key(&right));
        // Only one leaf is left, so it became the root.
        assert_eq!(layout.leaves().len(), 1);
        assert_eq!(layout.root(), layout.leaf_of(A).unwrap());
    }

    #[test]
    fn edge_drop_along_parent_axis_adds_a_sibling() {
        let (mut layout, left, _right) = two_leaves();
        layout.move_panel(C, left, DropZone::Left);
        let DockNode::Split { axis, children, .. } = layout.node(layout.root()) else {
            panic!("root should be a split")
        };
        assert_eq!(*axis, Axis::Horizontal);
        // C's leaf now sits before `left`; C's old leaf was removed.
        assert_eq!(children.len(), 2);
        assert_eq!(children[1], left);
        assert_eq!(tabs(&layout, children[0]), vec![C]);
    }

    #[test]
    fn edge_drop_across_parent_axis_wraps_target() {
        let (mut layout, left, right) = two_leaves();
        layout.move_panel(B, right, DropZone::Bottom);
        let DockNode::Split { children, .. } = layout.node(layout.root()) else {
            panic!("root should be a split")
        };
        assert_eq!(children[0], left);
        let DockNode::Split {
            axis,
            children: inner,
            ..
        } = layout.node(children[1])
        else {
            panic!("right side should be a vertical split")
        };
        assert_eq!(*axis, Axis::Vertical);
        assert_eq!(inner[0], right);
        assert_eq!(tabs(&layout, inner[1]), vec![B]);
    }

    #[test]
    fn dropping_a_lone_panel_on_its_own_leaf_does_nothing() {
        let (mut layout, _left, right) = two_leaves();
        assert!(!layout.move_panel(C, right, DropZone::Left));
        assert_eq!(tabs(&layout, right), vec![C]);
    }

    #[test]
    fn splitting_a_panel_off_its_own_leaf() {
        let (mut layout, left, _right) = two_leaves();
        assert!(layout.move_panel(B, left, DropZone::Right));
        assert_eq!(tabs(&layout, left), vec![A]);
        let new_leaf = layout.leaf_of(B).unwrap();
        assert_ne!(new_leaf, left);
        assert_eq!(layout.leaves().len(), 3);
    }
}
