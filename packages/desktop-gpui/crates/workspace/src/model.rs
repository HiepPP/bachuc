//! The split tree: tab groups in nested splits, each split with an axis and one ratio per child.
//! A port of the pane rules in `packages/app/src/stores/workspace-layout-actions.ts`. It holds no
//! GPUI state, so the rules are tested as plain functions.

use std::collections::HashMap;

/// The smallest share of a split that one child keeps (`MIN_SPLIT_SIZE`).
pub const MIN_SPLIT_SIZE: f32 = 0.1;
/// The width of a divider line in logical pixels.
pub const DIVIDER: f32 = 1.;
/// The divider's pointer band and its accent line while hovered or dragged. Both are odd, so
/// that centering them on the one-pixel line puts every edge on a whole pixel; an even width
/// lands on half pixels, and the band then draws 1 to 2 px off the line at 1x scale.
pub const DIVIDER_HIT: f32 = 11.;
pub const DIVIDER_HIGHLIGHT: f32 = 3.;

/// How far a centered band of `size` starts before the divider line's own start.
pub fn divider_inset(size: f32) -> f32 {
    (size - DIVIDER) / 2.
}

pub type NodeId = u64;

/// Where a split puts the new group: `Right` lays the children out in a row, `Down` in a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitAxis {
    Right,
    Down,
}

/// An item with a key that is unique in the whole layout.
pub trait Keyed {
    fn key(&self) -> &str;
}

impl Keyed for String {
    fn key(&self) -> &str {
        self
    }
}

#[derive(Clone, Debug)]
pub struct Group<T> {
    pub id: NodeId,
    pub items: Vec<T>,
    /// Index of the active item; meaningless while `items` is empty.
    pub active: usize,
}

impl<T: Keyed> Group<T> {
    pub fn active_item(&self) -> Option<&T> {
        self.items.get(self.active)
    }

    fn position(&self, key: &str) -> Option<usize> {
        self.items.iter().position(|item| item.key() == key)
    }
}

#[derive(Clone, Debug)]
pub struct Split<T> {
    pub id: NodeId,
    pub axis: SplitAxis,
    pub children: Vec<Node<T>>,
    /// One share per child; they sum to 1.
    pub ratios: Vec<f32>,
}

#[derive(Clone, Debug)]
pub enum Node<T> {
    Group(Group<T>),
    Split(Split<T>),
}

/// An axis-aligned rectangle in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The rectangles of a split's children, with a `DIVIDER` line between each pair.
pub fn child_rects(rect: Rect, axis: SplitAxis, ratios: &[f32]) -> Vec<Rect> {
    let dividers = ratios.len().saturating_sub(1) as f32 * DIVIDER;
    let length = match axis {
        SplitAxis::Right => rect.w,
        SplitAxis::Down => rect.h,
    };
    let available = (length - dividers).max(0.);
    let mut offset = 0.;
    ratios
        .iter()
        .map(|ratio| {
            let size = available * ratio;
            let child = match axis {
                SplitAxis::Right => Rect {
                    x: rect.x + offset,
                    w: size,
                    ..rect
                },
                SplitAxis::Down => Rect {
                    y: rect.y + offset,
                    h: size,
                    ..rect
                },
            };
            offset += size + DIVIDER;
            child
        })
        .collect()
}

/// Moves the divider after child `index` so that child takes `left` of the split, keeping the
/// pair's total and at least `MIN_SPLIT_SIZE` (or half the pair) on each side
/// (`computeResizeHandleSizes`).
pub fn resize_pair(ratios: &[f32], index: usize, left: f32) -> Vec<f32> {
    let mut next = ratios.to_vec();
    let (Some(&a), Some(&b)) = (ratios.get(index), ratios.get(index + 1)) else {
        return next;
    };
    let pair = a + b;
    if pair <= 0. {
        return next;
    }
    let min = MIN_SPLIT_SIZE.min(pair / 2.);
    let left = left.clamp(min, pair - min);
    next[index] = left;
    next[index + 1] = pair - left;
    next
}

fn normalize(ratios: &mut [f32]) {
    let total: f32 = ratios.iter().sum();
    if total <= 0. {
        let even = 1. / ratios.len().max(1) as f32;
        ratios.iter_mut().for_each(|ratio| *ratio = even);
    } else {
        ratios.iter_mut().for_each(|ratio| *ratio /= total);
    }
}

fn first_group<T>(node: &Node<T>) -> NodeId {
    match node {
        Node::Group(group) => group.id,
        Node::Split(split) => first_group(&split.children[0]),
    }
}

fn last_group<T>(node: &Node<T>) -> NodeId {
    match node {
        Node::Group(group) => group.id,
        Node::Split(split) => last_group(split.children.last().expect("a split has children")),
    }
}

fn collect_groups<'a, T>(node: &'a Node<T>, out: &mut Vec<&'a Group<T>>) {
    match node {
        Node::Group(group) => out.push(group),
        Node::Split(split) => split
            .children
            .iter()
            .for_each(|child| collect_groups(child, out)),
    }
}

fn group_ref<T>(node: &Node<T>, id: NodeId) -> Option<&Group<T>> {
    match node {
        Node::Group(group) => (group.id == id).then_some(group),
        Node::Split(split) => split.children.iter().find_map(|child| group_ref(child, id)),
    }
}

fn group_mut<T>(node: &mut Node<T>, id: NodeId) -> Option<&mut Group<T>> {
    match node {
        Node::Group(group) => (group.id == id).then_some(group),
        Node::Split(split) => split
            .children
            .iter_mut()
            .find_map(|child| group_mut(child, id)),
    }
}

fn split_ref<T>(node: &Node<T>, id: NodeId) -> Option<&Split<T>> {
    match node {
        Node::Group(_) => None,
        Node::Split(split) if split.id == id => Some(split),
        Node::Split(split) => split.children.iter().find_map(|child| split_ref(child, id)),
    }
}

fn split_mut<T>(node: &mut Node<T>, id: NodeId) -> Option<&mut Split<T>> {
    let Node::Split(split) = node else {
        return None;
    };
    if split.id == id {
        return Some(split);
    }
    split
        .children
        .iter_mut()
        .find_map(|child| split_mut(child, id))
}

fn is_group<T>(node: &Node<T>, id: NodeId) -> bool {
    matches!(node, Node::Group(group) if group.id == id)
}

/// Removes group `id` from the split that holds it and returns the group to focus instead: the
/// nearest group before it, else the one after. A split left with one child becomes that child,
/// so its divider goes away.
fn remove_group<T>(node: &mut Node<T>, id: NodeId) -> Option<NodeId> {
    let Node::Split(split) = node else {
        return None;
    };
    let Some(index) = split.children.iter().position(|child| is_group(child, id)) else {
        return split
            .children
            .iter_mut()
            .find_map(|child| remove_group(child, id));
    };
    split.children.remove(index);
    split.ratios.remove(index);
    normalize(&mut split.ratios);
    let neighbor = match index.checked_sub(1) {
        Some(before) => last_group(&split.children[before]),
        None => first_group(&split.children[0]),
    };
    if split.children.len() == 1 {
        let only = split.children.pop().expect("one child");
        *node = only;
    }
    Some(neighbor)
}

/// Replaces group `id` with a split of `axis` that holds it and `new`, or, when its parent split
/// already runs along `axis`, inserts `new` after it and halves its share.
fn insert_split<T>(
    node: &mut Node<T>,
    id: NodeId,
    axis: SplitAxis,
    new: Group<T>,
    split_id: NodeId,
) {
    if is_group(node, id) {
        let placeholder = Node::Group(Group {
            id: 0,
            items: Vec::new(),
            active: 0,
        });
        let old = std::mem::replace(node, placeholder);
        *node = Node::Split(Split {
            id: split_id,
            axis,
            children: vec![old, Node::Group(new)],
            ratios: vec![0.5, 0.5],
        });
        return;
    }
    let Node::Split(split) = node else {
        return;
    };
    if let Some(index) = split.children.iter().position(|child| is_group(child, id)) {
        if split.axis == axis {
            let half = split.ratios[index] / 2.;
            split.ratios[index] = half;
            split.ratios.insert(index + 1, half);
            split.children.insert(index + 1, Node::Group(new));
            return;
        }
        insert_split(&mut split.children[index], id, axis, new, split_id);
        return;
    }
    if let Some(child) = split
        .children
        .iter_mut()
        .find(|child| group_ref(child, id).is_some())
    {
        insert_split(child, id, axis, new, split_id);
    }
}

/// What `close` removed and the key that became active in the focused group, if it changed.
#[derive(Debug)]
pub struct Closed<T> {
    pub item: T,
    pub activated: Option<String>,
}

/// Groups of tabs in nested splits, with one focused group.
#[derive(Clone, Debug)]
pub struct Layout<T> {
    root: Node<T>,
    focused: NodeId,
    next_id: NodeId,
    /// The group shown alone. It is the focused group, or `None`; `settle` keeps it so.
    maximized: Option<NodeId>,
}

impl<T: Keyed> Default for Layout<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Keyed> Layout<T> {
    /// One empty group.
    pub fn new() -> Self {
        Self {
            root: Node::Group(Group {
                id: 1,
                items: Vec::new(),
                active: 0,
            }),
            focused: 1,
            next_id: 2,
            maximized: None,
        }
    }

    pub fn root(&self) -> &Node<T> {
        &self.root
    }

    pub fn focused(&self) -> NodeId {
        self.focused
    }

    /// The group shown alone, if any.
    pub fn maximized(&self) -> Option<NodeId> {
        self.maximized
    }

    /// Maximizes the focused group, or restores the layout when one is maximized. A layout with
    /// one group has nothing to maximize. Returns whether the state changed.
    pub fn toggle_maximize(&mut self) -> bool {
        if self.maximized.take().is_some() {
            return true;
        }
        if self.groups().len() < 2 {
            return false;
        }
        self.maximized = Some(self.focused);
        true
    }

    /// Keeps the maximize state valid after any change: it needs two groups to mean anything,
    /// and it follows the focus, so a tab opened or activated in a hidden group comes into view.
    fn settle(&mut self) {
        if self.maximized.is_none() {
            return;
        }
        self.maximized = (self.groups().len() > 1).then_some(self.focused);
    }

    /// Like `rects`, but a maximized group fills `rect` and every other node is absent.
    pub fn visible_rects(&self, rect: Rect) -> HashMap<NodeId, Rect> {
        match self.maximized {
            Some(group) => HashMap::from([(group, rect)]),
            None => self.rects(rect),
        }
    }

    pub fn group(&self, id: NodeId) -> Option<&Group<T>> {
        group_ref(&self.root, id)
    }

    pub fn split(&self, id: NodeId) -> Option<&Split<T>> {
        split_ref(&self.root, id)
    }

    /// The rectangle of every group and split when the root fills `rect`.
    pub fn rects(&self, rect: Rect) -> HashMap<NodeId, Rect> {
        fn walk<T>(node: &Node<T>, rect: Rect, out: &mut HashMap<NodeId, Rect>) {
            match node {
                Node::Group(group) => {
                    out.insert(group.id, rect);
                }
                Node::Split(split) => {
                    out.insert(split.id, rect);
                    for (child, child_rect) in
                        split
                            .children
                            .iter()
                            .zip(child_rects(rect, split.axis, &split.ratios))
                    {
                        walk(child, child_rect, out);
                    }
                }
            }
        }
        let mut out = HashMap::new();
        walk(&self.root, rect, &mut out);
        out
    }

    /// Every group in reading order.
    pub fn groups(&self) -> Vec<&Group<T>> {
        let mut out = Vec::new();
        collect_groups(&self.root, &mut out);
        out
    }

    fn focused_group(&self) -> &Group<T> {
        self.group(self.focused).expect("the focused group exists")
    }

    /// The active key of the focused group.
    pub fn active_key(&self) -> Option<&str> {
        self.focused_group().active_item().map(Keyed::key)
    }

    /// The group and index that hold `key`.
    pub fn find(&self, key: &str) -> Option<(NodeId, usize)> {
        self.groups()
            .into_iter()
            .find_map(|group| group.position(key).map(|index| (group.id, index)))
    }

    pub fn item(&self, key: &str) -> Option<&T> {
        let (group, index) = self.find(key)?;
        self.group(group)?.items.get(index)
    }

    fn take_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Focuses a group. Returns whether the focus moved.
    pub fn focus(&mut self, group: NodeId) -> bool {
        if group == self.focused || self.group(group).is_none() {
            return false;
        }
        self.focused = group;
        self.settle();
        true
    }

    /// Opens `item` at the end of the focused group and activates it. An item whose key is open
    /// replaces the open one in place, and its group takes the focus.
    pub fn open(&mut self, item: T) {
        if let Some((group, index)) = self.find(item.key()) {
            let group_ref = group_mut(&mut self.root, group).expect("found");
            group_ref.items[index] = item;
            group_ref.active = index;
            self.focused = group;
            self.settle();
            return;
        }
        let focused = self.focused;
        let group = group_mut(&mut self.root, focused).expect("the focused group exists");
        group.items.push(item);
        group.active = group.items.len() - 1;
    }

    /// Replaces the open item with the same key without activating it. Returns false when the
    /// key is not open.
    pub fn update(&mut self, item: T) -> bool {
        let Some((group, index)) = self.find(item.key()) else {
            return false;
        };
        group_mut(&mut self.root, group).expect("found").items[index] = item;
        true
    }

    /// Activates the item at `index` of a group and focuses that group.
    pub fn activate(&mut self, group: NodeId, index: usize) -> bool {
        let Some(group_ref) = group_mut(&mut self.root, group) else {
            return false;
        };
        if index >= group_ref.items.len() {
            return false;
        }
        group_ref.active = index;
        self.focused = group;
        self.settle();
        true
    }

    /// Activates the item at the 0-based `index` of the focused group.
    pub fn activate_index(&mut self, index: usize) -> bool {
        self.activate(self.focused, index)
    }

    /// Removes the item at `index` and picks the next active item as Electron does: the item after
    /// it, else the one before. Removes the group when it empties and is not the only one.
    fn remove_at(&mut self, group: NodeId, index: usize) -> T {
        let group_ref = group_mut(&mut self.root, group).expect("group exists");
        let item = group_ref.items.remove(index);
        if index < group_ref.active || (index == group_ref.active && index == group_ref.items.len())
        {
            group_ref.active = group_ref.active.saturating_sub(1);
        }
        if group_ref.items.is_empty()
            && matches!(self.root, Node::Split(_))
            && let Some(neighbor) = remove_group(&mut self.root, group)
        {
            // Electron drops the maximized pane when it goes away; the others stay hidden.
            if self.maximized == Some(group) {
                self.maximized = None;
            }
            if self.focused == group {
                self.focused = neighbor;
            }
        }
        self.settle();
        item
    }

    /// Closes the item with `key`.
    pub fn close(&mut self, key: &str) -> Option<Closed<T>> {
        let (group, index) = self.find(key)?;
        let before = self.active_key().map(str::to_owned);
        let item = self.remove_at(group, index);
        let after = self.active_key().map(str::to_owned);
        Some(Closed {
            item,
            activated: after.filter(|after| before.as_deref() != Some(after)),
        })
    }

    /// Closes the active item of the focused group.
    pub fn close_active(&mut self) -> Option<Closed<T>> {
        let key = self.active_key()?.to_owned();
        self.close(&key)
    }

    /// Adds an empty group next to the focused group along `axis`, focuses it, and returns it.
    pub fn split_focused(&mut self, axis: SplitAxis) -> NodeId {
        let id = self.take_id();
        let split_id = self.take_id();
        let new = Group {
            id,
            items: Vec::new(),
            active: 0,
        };
        let focused = self.focused;
        insert_split(&mut self.root, focused, axis, new, split_id);
        self.focused = id;
        // The new group is the one the user is about to fill, so it must show.
        self.maximized = None;
        id
    }

    /// Moves `key` to `slot` of group `to`. `slot` counts the target's items as they show now,
    /// including the moved item when it stays in its group, and `items.len()` means the end. The
    /// moved item becomes active and its new group takes the focus. A source group left empty is
    /// removed. Returns whether anything moved.
    pub fn move_item(&mut self, key: &str, to: NodeId, slot: usize) -> bool {
        let Some((from, index)) = self.find(key) else {
            return false;
        };
        let Some(target) = self.group(to) else {
            return false;
        };
        let slot = slot.min(target.items.len());
        if from == to {
            let target_index = if slot > index { slot - 1 } else { slot };
            let group = group_mut(&mut self.root, to).expect("checked");
            let item = group.items.remove(index);
            group.items.insert(target_index, item);
            group.active = target_index;
            self.focused = to;
            self.settle();
            return target_index != index;
        }
        let item = self.remove_at(from, index);
        let group = group_mut(&mut self.root, to).expect("the target is not the source");
        group.items.insert(slot, item);
        group.active = slot;
        self.focused = to;
        self.settle();
        true
    }

    /// Moves the divider after child `index` of `split` so that child takes `left` of the split.
    pub fn resize(&mut self, split: NodeId, index: usize, left: f32) -> bool {
        let Some(split) = split_mut(&mut self.root, split) else {
            return false;
        };
        let next = resize_pair(&split.ratios, index, left);
        let changed = next != split.ratios;
        split.ratios = next;
        changed
    }
}
