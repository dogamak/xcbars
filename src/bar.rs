use item_state::ItemState;
use bar_builder::UpdateStream;
use std::rc::Rc;
use xcb::{Connection, Rectangle, Window};
use futures::stream::{Merge, MergedItem};
use futures::{Future, Stream};
use xcb_event_stream::XcbEventStream;
use error::*;
use std::error::Error;
use xcb;
use component::Slot;

type UpdateAndEventStream = Merge<UpdateStream, XcbEventStream>;

/// Struct that contains everything needed to run the bar.
pub struct Bar {
    pub center_items: Vec<ItemState>,
    pub conn: Rc<Connection>,
    pub foreground: u32,
    pub geometry: Rectangle,
    pub item_positions: Vec<(u16, u16)>,
    pub left_items: Vec<ItemState>,
    pub right_items: Vec<ItemState>,
    pub stream: Option<UpdateAndEventStream>,
    pub window: Window,
    pub inner_padding: u16,
}

impl Bar {
    /// Returns `self.stream` without borrowing or consuming `self`.
    /// Panics if called twice.
    fn get_stream(&mut self) -> UpdateAndEventStream {
        ::std::mem::replace(&mut self.stream, None).unwrap()
    }

    /// Launch and run the bar.
    pub fn run(mut self) -> Box<Future<Item = (), Error = ()>> {
        let future = self.get_stream()
            .map_err(|e| ::error::Error::with_chain(e, ErrorKind::ItemError))
            .for_each(move |item| -> Result<()> {
                let (_, update) = match item {
                    MergedItem::First(update) => (None, Some(update)),
                    MergedItem::Second(event) => (Some(event), None),
                    MergedItem::Both(update, event) => (Some(event), Some(update)),
                };

                if let Some(update) = update {
                    let size_changed;

                    // Figure out if the component has chanaged
                    // it's size since the last update.
                    // TODO: Move to a function
                    {
                        let slot_items = match update.slot {
                            Slot::Left => &mut self.left_items,
                            Slot::Center => &mut self.center_items,
                            Slot::Right => &mut self.right_items,
                        };
                        slot_items[update.index].update(update.value)?;

                        let width = slot_items[update.index].get_content_width();
                        size_changed = self.item_positions[update.id].1 != width;
                    }

                    // Redraw only neccessary stuff
                    match update.slot {
                        Slot::Center => self.redraw_center()?,
                        Slot::Left => self.redraw_left(size_changed, update.index)?,
                        Slot::Right => self.redraw_right(size_changed, update.index)?,
                    }
                }
                Ok(())
            })
            .map_err(|err| {
                println!("Error occurred: {}", err);
                let mut cause = err.cause();
                while let Some(err) = cause {
                    println!("  Caused by: {}", err);
                    cause = err.cause();
                }
            });

        Box::new(future)
    }

    /// Redraws components in the center slot.
    /// Unforunately centering means that all components
    /// must be redrawn if even one of them changes size.
    fn redraw_center(&mut self) -> Result<()> {
        // Get future width of all center components
        let width_all: u16 = self.center_items
            .iter()
            .map(|item| item.get_content_width())
            .sum();

        // Draw blank background to prevent leftovers after shrinkage
        // Only does this when component width has shrunk
        let old_width_all: u16 = self.center_items
            .iter()
            .map(|item| self.item_positions[item.get_id()].1)
            .sum();
        if width_all < old_width_all {
            if let Some(first) = self.center_items.first() {
                let old_start = self.item_positions[first.get_id()].0;
                self.paint_bg(old_start, old_start + old_width_all)?;
            }
        }

        let mut pos = (self.geometry.width()) / 2 - width_all / 2;

        for item in &self.center_items {
            self.item_positions[item.get_id()].0 = pos;
            self.draw_item(item, pos)?;
            self.item_positions[item.get_id()].1 = item.get_content_width();
            pos += item.get_content_width();
        }

        Ok(())
    }

    /// Pretty much the same as `self.redraw_left` but with `left` replaced with `right`.
    /// The order in which the items are gone through is reversed.
    fn redraw_right(&mut self, size_changed: bool, index: usize) -> Result<()> {
        let mut pos = self.geometry.width() - self.inner_padding;

        for n in 0..self.right_items.len() {
            let item = &self.right_items[self.right_items.len() - n - 1];

            pos -= item.get_content_width();

            if n < self.right_items.len() - index - 1 {
                continue;
            }

            if size_changed {
                let mut bg_start = pos;
                let bg_end = pos + item.get_content_width();

                if n == self.right_items.len() - 1 {
                    let old_start = self.item_positions[item.get_id()].0 as u16;
                    if old_start < bg_start && old_start > 0 {
                        bg_start = old_start;
                    }
                }

                self.paint_bg(bg_start, bg_end)?;
            }

            self.item_positions[item.get_id()].0 = pos;
            self.item_positions[item.get_id()].1 = item.get_content_width();
            self.draw_item(item, pos)?;

            if !size_changed {
                break;
            }
        }

        Ok(())
    }

    /// Redraw only needed items in the right slot.
    /// Symmetric to `self.redraw_left`.
    ///
    /// If the component hasn't changed it's size, it doesn't affect
    /// any other components and we can get away with just painting
    /// the one component.
    ///
    /// However if the component has changed it's size, we must also
    /// redraw every component on the right of it. If the item has shrunk
    /// we must also repaint the exposed background.
    fn redraw_left(&mut self, size_changed: bool, index: usize) -> Result<()> {
        let mut pos = self.inner_padding;

        for n in 0..self.left_items.len() {
            let item = &self.left_items[n];

            if n < index {
                pos += item.get_content_width();
                continue;
            }

            if size_changed {
                let bg_start = pos;
                let mut bg_end = pos + item.get_content_width();

                if n == self.left_items.len() - 1 {
                    let old_end =
                        self.item_positions[item.get_id()].0 + self.item_positions[item.get_id()].1;
                    if bg_end < old_end {
                        bg_end = old_end;
                    }
                }

                self.paint_bg(bg_start, bg_end)?;
            }

            self.item_positions[item.get_id()].0 = pos;
            self.item_positions[item.get_id()].1 = item.get_content_width();
            self.draw_item(item, pos)?;

            if !size_changed {
                break;
            }

            pos += item.get_content_width();
        }

        Ok(())
    }

    /// Copies the item's pixmap to the window.
    fn draw_item(&self, item: &ItemState, pos: u16) -> Result<()> {
        if !item.is_ready() {
            return Ok(());
        }

        try_xcb!(
            xcb::copy_area_checked,
            "failed to copy pixmap",
            &self.conn,
            item.get_pixmap(),
            self.window,
            self.foreground,
            0,
            0,
            pos as i16,
            0,
            item.get_content_width() as u16,
            self.geometry.height()
        );

        Ok(())
    }

    /// Draws the background starting at point a on the x-axis until point b.
    fn paint_bg(&self, a: u16, b: u16) -> Result<()> {
        try_xcb!(
            xcb::poly_fill_rectangle,
            "failed to draw background",
            &self.conn,
            self.window,
            self.foreground,
            &[Rectangle::new(a as i16, 0, b - a, self.geometry.height())]
        );

        Ok(())
    }
}
