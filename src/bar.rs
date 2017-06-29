use item_state::ItemState;
use bar_builder::UpdateStream;
use std::rc::Rc;
use xcb::{Connection, Rectangle, Window, Pixmap};
use futures::stream::{Merge, MergedItem};
use futures::{Future, Stream};
use xcb_event_stream::XcbEventStream;
use error::*;
use std::error::Error;
use xcb;
use component::{Slot, ComponentStateExt};
use component_context::ComponentContext;

type UpdateAndEventStream = Merge<UpdateStream, XcbEventStream>;

/// Struct that contains everything needed to run the bar.
pub struct Bar {
    pub left_items: Vec<(Option<ComponentContext>, Box<ComponentStateExt>)>,
    pub center_items: Vec<(Option<ComponentContext>, Box<ComponentStateExt>)>,
    pub right_items: Vec<(Option<ComponentContext>, Box<ComponentStateExt>)>,
    pub conn: Rc<Connection>,
    pub foreground: u32,
    pub geometry: Rectangle,
    pub window: Window,
}

impl Bar {
    /// Returns `self.stream` without borrowing or consuming `self`.
    /// Panics if called twice.
    fn get_stream(&mut self) -> UpdateAndEventStream {
        unimplemented!();
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
                        slot_items[update.index].1.update(Box::new(update.value))?;

                        let width = slot_items[update.index].1.get_preferred_width();
                        size_changed = match slot_items[update.id].0 {
                            None => true,
                            Some(ref state) => state.width() != width,
                        };
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
        let width_all: u16 = self.center_items
            .iter()
            .map(|item| match item.0 {
                None => 0,
                Some(ref state) => state.width(),
            })
            .sum();

        let mut pos = (self.geometry.width()) / 2 - width_all / 2;

        for &mut (ref mut context, ref state) in self.center_items.iter_mut() {
            if let Some(ref mut context) = *context {
                let width = state.get_preferred_width();
                context.update(pos, width);
                // self.draw_item(item, pos)?;
                pos += width;
            } else {
                unreachable!();
            }
        }

        Ok(())
    }

    /// Pretty much the same as `self.redraw_left` but with `left` replaced with `right`.
    /// The order in which the items are gone through is reversed.
    fn redraw_right(&mut self, size_changed: bool, index: usize) -> Result<()> {
        let mut pos = self.geometry.width();
        for n in 0..self.right_items.len() {
            let right_item_count = self.right_items.len();
            let mut draw_bg_info = None;
            let pixmap;
            let width;

            {
                let &mut (ref mut context, ref state) = &mut self.right_items[right_item_count - n - 1];
                let context = match *context {
                    Some(ref mut context) => context,
                    None => unreachable!(),
                };

                width = state.get_preferred_width();
                pos -= width;

                if n < right_item_count - index - 1 {
                    continue;
                }

                if size_changed {
                    let mut bg_start = pos;
                    let bg_end = pos + width;

                    if n == right_item_count - 1 {
                        let old_start = context.position();
                        if old_start < bg_start {
                            bg_start = old_start;
                        }
                    }

                    draw_bg_info = Some((bg_start, bg_end));
                }

                context.update(pos, width);

                pixmap = context.pixmap();
            }

            if let Some((bg_start, bg_end)) = draw_bg_info {
                self.paint_bg(bg_start, bg_end)?;
            }

            self.draw_item(pixmap, pos, width)?;

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
        let pos = 0;
        let left_item_count = self.left_items.len();

        for n in 0..self.left_items.len() {
            let mut draw_bg_info = None;
            let pixmap;
            let width;

            {
                let &mut (ref mut context, ref state) = &mut self.left_items[n];
                let context = match *context {
                    Some(ref mut context) => context,
                    None => unreachable!(),
                };

                width = state.get_preferred_width();

                if n < index {
                    continue;
                }

                if size_changed {
                    let bg_start = pos;
                    let mut bg_end = pos + width;

                    if n == left_item_count - 1 {
                        let old_end = context.position() + context.width();
                        if bg_end < old_end {
                            bg_end = old_end;
                        }
                    }

                    context.update(pos, width)?;
                    draw_bg_info = Some((bg_start, bg_end));
                }

                pixmap = context.pixmap();
            }

            if let Some((bg_start, bg_end)) = draw_bg_info {
                self.paint_bg(bg_start, bg_end)?;
            }

            self.draw_item(pixmap, pos, width)?;

            if !size_changed {
                break;
            }
        }

        Ok(())
    }

    /// Copies the item's pixmap to the window.
    fn draw_item(&self, pixmap: Pixmap, pos: u16, width: u16) -> Result<()> {
        try_xcb!(
            xcb::copy_area_checked,
            "failed to copy pixmap",
            &self.conn,
            pixmap,
            self.window,
            self.foreground,
            0,
            0,
            pos as i16,
            0,
            width,
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
