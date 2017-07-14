use bar_builder::Color;
use std::rc::Rc;
use xcb::{self, Visualtype, Screen, Connection, Rectangle, Window, Pixmap};
use futures::{Poll, Async, Future};
use error::*;
use error::Error;
use pango::FontDescription;
use component::{Slot, ComponentStateWrapperExt};
use component_context::ComponentContext;

type ComponentInfo = (ComponentContext, Box<ComponentStateWrapperExt>);

/// Struct for holding XCB related information required by various components.
pub struct XcbContext {
    pub conn: Connection,
    pub window: Window,
    pub screen_index: usize,
    pub visualtype: Visualtype,
}

impl XcbContext {
    #[inline]
    pub fn screen<'s>(&'s self) -> Screen<'s> {
        self.conn.get_setup().roots().nth(self.screen_index).unwrap()
    } 
}

/// Struct for storing basic information about the Bar.
/// This is handed down for the Components.
pub struct BarInfo {
    pub fg: Color,
    pub bg: Color,
    pub font: FontDescription,
}

/// Struct that contains everything needed to run the bar.
pub struct Bar {
    pub xcb_ctx: Rc<XcbContext>,
    pub components: Vec<ComponentInfo>,
    pub left_component_count: usize,
    pub center_component_count: usize,
    pub right_component_count: usize,
    pub foreground: u32,
    pub geometry: Rectangle,
}

impl Bar {
    fn handle_redraw(&mut self, index: usize, width_changed: bool) -> Result<()> {
        if index < self.left_component_count {
            self.redraw_left(width_changed, index)
        } else if index < self.left_component_count + self.center_component_count {
            self.redraw_center()
        } else {
            self.redraw_right(width_changed, index)
        }
    }

    #[inline]
    fn slot_items_mut(&mut self, slot: Slot) -> &mut [ComponentInfo] {
        match slot {
            Slot::Left => &mut self.components[..self.left_component_count],
            Slot::Center => &mut self.components[self.left_component_count..self.left_component_count+self.center_component_count],
            Slot::Right => &mut self.components[self.left_component_count+self.center_component_count..],
        }
    }

    /// Launch and run the bar.
    /* pub fn run(mut self) -> Box<Future<Item = (), Error = ()>> {
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
    } */

    /// Redraws components in the center slot.
    /// Unforunately centering means that all components
    /// must be redrawn if even one of them changes size.
    fn redraw_center(&mut self) -> Result<()> {
        let width_all: u16 = self.slot_items_mut(Slot::Center)
            .iter()
            .map(|item| item.0.width().unwrap_or(0))
            .sum();

        let mut pos = (self.geometry.width()) / 2 - width_all / 2;

        let mut center = self.slot_items_mut(Slot::Center).len();
        for n in 0..self.center_component_count {
            let pixmap;
            let width;
            {
                let &mut (ref mut context, ref state) = &mut self.components[self.left_component_count + n];
                width = state.get_preferred_width();
                context.position = Some(pos);
                pixmap = context.pixmap().unwrap();
            }
            self.draw_item(pixmap, pos, width)?;
            pos += width;
        }

        Ok(())
    }

    /// Pretty much the same as `self.redraw_left` but with `left` replaced with `right`.
    /// The order in which the items are gone through is reversed.
    fn redraw_right(&mut self, size_changed: bool, index: usize) -> Result<()> {
        let mut pos = self.geometry.width();
        let right_item_count = self.slot_items_mut(Slot::Right).len();

        for n in 0..right_item_count {
            let mut draw_bg_info = None;
            let pixmap;
            let width;

            {
                let &mut (ref mut context, ref state) = &mut self.slot_items_mut(Slot::Right)[right_item_count - n - 1];

                width = state.get_preferred_width();
                pos -= width;

                if n < right_item_count - index {
                    continue;
                }

                if size_changed {
                    let mut bg_start = pos;
                    let bg_end = pos + width;

                    if n == right_item_count - 1 {
                        if let Some(old_start) = context.position {
                            if old_start < bg_start {
                                bg_start = old_start;
                            }
                        }
                    }

                    draw_bg_info = Some((bg_start, bg_end));
                }

                context.position = Some(pos);

                pixmap = context.pixmap().unwrap();
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
        let mut pos = 0;
        let left_item_count = self.slot_items_mut(Slot::Left).len();

        for n in 0..left_item_count {
            let mut draw_bg_info = None;
            let pixmap;
            let width;

            {
                let &mut (ref mut context, ref state) = &mut self.slot_items_mut(Slot::Left)[n];

                width = state.get_preferred_width();

                if n < index {
                    pos += width;
                    continue;
                }

                if size_changed {
                    let bg_start = pos;
                    let mut bg_end = pos + width;

                    if n == left_item_count - 1 {
                        if let Some(old_start) = context.position {
                            let old_end = old_start + context.width().unwrap();
                            if bg_end < old_end {
                                bg_end = old_end;
                            }
                        }
                    }

                    context.position = Some(pos);
                    draw_bg_info = Some((bg_start, bg_end));
                }

                pixmap = context.pixmap().unwrap();
            }

            if let Some((bg_start, bg_end)) = draw_bg_info {
                self.paint_bg(bg_start, bg_end)?;
            }

            self.draw_item(pixmap, pos, width)?;

            if !size_changed {
                break;
            }

            pos += width;
        }

        Ok(())
    }

    /// Copies the item's pixmap to the window.
    fn draw_item(&self, pixmap: Pixmap, pos: u16, width: u16) -> Result<()> {
        try_xcb!(
            xcb::copy_area_checked,
            "failed to copy pixmap",
            &self.xcb_ctx.conn,
            pixmap,
            self.xcb_ctx.window,
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
            &self.xcb_ctx.conn,
            self.xcb_ctx.window,
            self.foreground,
            &[Rectangle::new(a as i16, 0, b - a, self.geometry.height())]
        );

        Ok(())
    }
}

impl Future for Bar {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<(), Error> {
        println!("Poll Bar");
        let mut updated = vec![];
        let mut not_ready = false;
        
        for (index, &mut (ref mut context, ref mut state)) in self.components.iter_mut().enumerate() {
            let result = state.poll();
            println!("{:?}", result);
            match result {
                Ok(Async::Ready(Some(()))) => updated.push(index),
                Ok(Async::NotReady) => not_ready = true,
                Err(e) => return Err(e),
                _ => continue,
            }
        }

        for index in updated {
            let width_changed;
            
            {
                let &mut (ref mut context, ref mut state) =
                    &mut self.components[index];

                let width = state.get_preferred_width();

                width_changed = match context.width() {
                    Some(prev_width) => width != prev_width,
                    None => true,
                };

                context.update_width(width)?;
                state.render(
                    context.surface().unwrap(),
                    width,
                    self.geometry.height())?;
            }

            self.handle_redraw(index, width_changed)?;
        }

        if not_ready {
            Ok(Async::NotReady)
        } else {
            let result = self.poll();
            println!("Again: {:?}", result);
            result
        }
    }
}
