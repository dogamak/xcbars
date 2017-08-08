# xcbars [![Build Status](https://img.shields.io/travis/dogamak/xcbars/master.svg?style=flat-square)](https://travis-ci.org/dogamak/xcbars)
A bar library created with rust and xcb.

## Example
```rust
let down_speed = NetworkUsage {
    interface: "wlp58s0".to_string(),
    .. Default::default()
};

BarBuilder::new()
    .geometry(Geometry::Relative {
        position: Position::Top,
        height: 20,
        padding_x: 5,
        padding_y: 5,
    })
    .background(Color::new(1.0, 0.5, 0.5))
    .foreground(Color::new(1., 1., 1.))
    .font("Inconsolata 14")
    .add_component(Slot::Left, Counter {
        start: 123,
        step: 2,
    })
    .add_component(Slot::Center, Pipe {
        command: "date",
    })
    .add_component(Slot::Right, composite!("Down: ", down_speed))
    .run().unwrap();
```
