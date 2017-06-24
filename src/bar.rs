use item_state::ItemState;
use bar_builder::{
    UpdateStream,
};
use std::rc::Rc;
use xcb::{
    Connection,
    Rectangle,
    Window,
};
use futures::stream::{Merge, MergedItem};
use futures::{Future, Stream};
use xcb_event_stream::XcbEventStream;
use error::*;
use std::error::Error;
use xcb;
use component::Slot;

type UpdateAndEventStream = Merge<UpdateStream, XcbEventStream>;

pub struct Bar {
    pub center_items: Vec<ItemState>,
    pub conn: Rc<Connection>,
    pub foreground: u32,
    pub geometry: Rectangle,
    pub item_positions: Vec<(i16, u16)>,
    pub left_items: Vec<ItemState>,
    pub right_items: Vec<ItemState>,
    pub stream: Option<UpdateAndEventStream>,
    pub window: Window,
}

impl Bar {
    fn get_stream(&mut self) -> UpdateAndEventStream {
        ::std::mem::replace(&mut self.stream, None).unwrap()
    }
    
    pub fn run(mut self) -> Box<Future<Item=(), Error=()>> {
        let future = self.get_stream()
            .map_err(|_| ErrorKind::ItemError.into())
            .for_each(move |item| -> Result<()> {
                let (_, update) = match item {
                    MergedItem::First(update) => (None, Some(update)),
                    MergedItem::Second(event) => (Some(event), None),
                    MergedItem::Both(update, event) => (Some(event), Some(update)),
                };

                if let Some(update) = update {
                    let size_changed;
                    let width;

                    {
                        let slot_items = match update.slot {
                            Slot::Left => &mut self.left_items,
                            Slot::Center => &mut self.center_items,
                            Slot::Right => &mut self.right_items,
                        };
                        slot_items[update.index].update(update.value)?;

                        width = slot_items[update.index].get_content_width();
                        size_changed = self.item_positions[update.id].1 != width;
                    }

                    if size_changed {
                        self.paint_bg(update.id)?;
                    }

                    self.item_positions[update.id].1 = width;

                    match update.slot {
                        Slot::Center => self.redraw_center()?,
                        Slot::Left => self.redraw_end(false, size_changed, update.index)?,
                        Slot::Right => self.redraw_end(true, size_changed, update.index)?,
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

    fn redraw_center(&mut self) -> Result<()> {
        let width_all: i16 = self.center_items.iter()
            .map(|item| item.get_content_width() as i16)
            .sum();

        let mut pos = (self.geometry.width() as i16)/2-width_all/2;

        for item in self.center_items.iter() {
            self.item_positions[item.get_id()].0 = pos;
            self.draw_item(item, pos)?;
            pos += item.get_content_width() as i16;
        }

        Ok(())
    }

    fn redraw_end(&mut self, mut side: bool, size_changed: bool, index: usize) -> Result<()> {
        let (start, direction, items) = match side {
            false => (0, 1, &self.left_items),
            true => (self.geometry.width() as i16, -1, &self.right_items),
        };

        let (before, mut after) = items.split_at(index);

        if !size_changed {
            after = &after[0..1];
        }

        let mut pos: i16 = before.iter()
            .map(|item| item.get_content_width() as i16).sum();

        for item in after.iter() {
            if side {
                pos += item.get_content_width() as i16;
            }
            side = true;
            self.item_positions[item.get_id()].0 = pos;
            self.draw_item(item, start+(pos*direction))?;
        }

        self.conn.flush();
        
        Ok(())
    }

    fn draw_item(&self, item: &ItemState, pos: i16) -> Result<()> {
        if !item.is_ready() {
            return Ok(());
        }
        self.paint_bg(item.get_id())?;
        try_xcb!(xcb::copy_area_checked, "failed to copy pixmap",
            &self.conn,
            item.get_pixmap(),
            self.window,
            self.foreground,
            0, 0,
            pos, 0,
            item.get_content_width() as u16,
            self.geometry.height());
        Ok(())
    }

    fn paint_bg(&self, index: usize) -> Result<()> {
        let x = self.item_positions[index].0;
        let w = self.item_positions[index].1;

        try_xcb!(xcb::poly_fill_rectangle, "failed to draw background",
            &self.conn,
            self.window,
            self.foreground,
            &[Rectangle::new(x, 0, w, self.geometry.height())]);

        Ok(())
    }
}
