use crate::converters::Converter;
use crate::ConverterFactory;
use crate::LabelGetter;
use std::cell::RefCell;
use std::rc::Rc;

type StackItemPtr = Rc<RefCell<StackItem>>;

struct StackItem {
    label: String,
    parent: Option<StackItemPtr>,
    duration: u64,
    last_start: u64,
    last_stop: u64,
    children: std::collections::HashMap<String, StackItemPtr>,
}

impl StackItem {
    pub fn new(label: String, parent: Option<StackItemPtr>, start: u64, stop: u64) -> StackItem {
        StackItem {
            label,
            parent,
            duration: 0,
            last_start: start,
            last_stop: stop,
            children: std::collections::HashMap::<String, StackItemPtr>::new(),
        }
    }

    pub fn new_root() -> StackItem {
        StackItem::new(String::new(), None, 0, std::u64::MAX)
    }

    pub fn update_last_range(&mut self, start_ts: u64, stop_ts: u64) {
        self.duration += stop_ts - start_ts;
        self.last_start = start_ts;
        self.last_stop = stop_ts;
    }
}

struct EventItem {
    label: String,
    thread_id: u32,
    start_ts: u64,
    stop_ts: u64,
}

struct ThreadStack {
    root_item: StackItemPtr,
    current_item: StackItemPtr,
}

impl ThreadStack {
    pub fn new() -> ThreadStack {
        let root_item = Rc::new(RefCell::new(StackItem::new_root()));
        ThreadStack {
            current_item: Rc::clone(&root_item),
            root_item,
        }
    }

    fn is_parent_of(&self, stack_item: &StackItemPtr, event_item: &EventItem) -> bool {
        let stack_item = stack_item.borrow();
        stack_item.last_start <= event_item.start_ts && stack_item.last_stop >= event_item.stop_ts
    }

    pub fn add_item(&mut self, item: &EventItem) {
        while !self.is_parent_of(&self.current_item, item) {
            let parent = Rc::clone(&self.current_item.borrow().parent.as_ref().unwrap());
            self.current_item = parent;
        }

        if !self
            .current_item
            .borrow()
            .children
            .contains_key(&item.label)
        {
            let current_item_cpy = Rc::clone(&self.current_item);
            self.current_item.borrow_mut().children.insert(
                item.label.clone(),
                Rc::new(RefCell::new(StackItem::new(
                    item.label.clone(),
                    Some(current_item_cpy),
                    item.start_ts,
                    item.stop_ts,
                ))),
            );
        }

        let new_current_item = Rc::clone(&self.current_item.borrow().children[&item.label]);
        new_current_item
            .borrow_mut()
            .update_last_range(item.start_ts, item.stop_ts);
        self.current_item = new_current_item;
    }
}

struct FlamegraphConverter {
    writable: Box<std::io::Write>,
    stacks: std::collections::HashMap<u32, ThreadStack>,
    items: std::vec::Vec<EventItem>,
    label_getter: LabelGetter,
}

impl FlamegraphConverter {
    pub fn new(writable: Box<std::io::Write>, label_getter: LabelGetter) -> FlamegraphConverter {
        FlamegraphConverter {
            writable,
            label_getter,
            items: vec![],
            stacks: std::collections::HashMap::<u32, ThreadStack>::new(),
        }
    }

    pub fn generate_flamegraph(&mut self) {
        self.items.sort_by_key(|k| k.start_ts);

        for item in &self.items {
            self.stacks
                .entry(item.thread_id)
                .or_insert_with(ThreadStack::new)
                .add_item(item)
        }

        let super_root = Rc::new(RefCell::new(StackItem::new_root()));
        for stack in self.stacks.values() {
            self.merge_stacks(&super_root, &stack.root_item);
        }
        HTMLFlameGraphWritter::new(&mut self.writable).write_flamegraph(&super_root);
    }

    fn merge_stacks(&self, super_stack: &StackItemPtr, stack: &StackItemPtr) {
        for child in stack.borrow().children.values() {
            if super_stack
                .borrow()
                .children
                .contains_key(&child.borrow().label)
            {
                super_stack.borrow().children[&child.borrow().label]
                    .borrow_mut()
                    .duration += child.borrow().duration;
                self.merge_stacks(
                    &super_stack.borrow().children[&child.borrow().label],
                    &child,
                );
            } else {
                super_stack
                    .borrow_mut()
                    .children
                    .insert(child.borrow().label.clone(), Rc::clone(child));
            }
        }
    }
}

impl Converter for FlamegraphConverter {
    fn process_event(
        &mut self,
        event: &hawktracer_parser::Event,
        _reg: &hawktracer_parser::EventKlassRegistry,
    ) -> Result<(), Box<std::error::Error>> {
        let timestamp = event.get_value_u64("timestamp");
        let duration = event.get_value_u64("duration");
        let thread_id = event.get_value_u32("thread_id");
        let (_, label) = self.label_getter.get_label(event);

        if timestamp.is_err() || duration.is_err() || label.is_none() || thread_id.is_err() {
            return Ok(());
        }

        let item = EventItem {
            label: label.unwrap().clone(),
            thread_id: thread_id.unwrap(),
            start_ts: *timestamp.as_ref().unwrap(),
            stop_ts: timestamp.unwrap() + duration.unwrap(),
        };

        self.items.push(item);

        Ok(())
    }
}

impl Drop for FlamegraphConverter {
    fn drop(&mut self) {
        self.generate_flamegraph();
    }
}

pub struct FlamegraphConverterFactory {}

impl ConverterFactory for FlamegraphConverterFactory {
    fn construct(
        &self,
        writable: Box<std::io::Write>,
        label_getter: LabelGetter,
    ) -> Box<Converter> {
        Box::new(FlamegraphConverter::new(writable, label_getter))
    }

    fn get_name(&self) -> &str {
        "flamegraph"
    }
}

struct HTMLFlameGraphWritter<'a> {
    writable: &'a mut std::io::Write,
}

impl<'a> HTMLFlameGraphWritter<'a> {
    pub fn new(writable: &'a mut std::io::Write) -> HTMLFlameGraphWritter<'a> {
        HTMLFlameGraphWritter { writable }
    }

    pub fn write_flamegraph(&mut self, root_item: &StackItemPtr) -> std::io::Result<()> {
        self.write_header()?;
        self.write_stack_item(&root_item)?;
        self.write_footer()
    }

    fn is_root(&self, item: &StackItemPtr) -> bool {
        item.borrow().parent.is_none()
    }

    fn write_stack_item(&mut self, item: &StackItemPtr) -> std::io::Result<()> {
        if !self.is_root(item) {
            self.writable.write_fmt(format_args!(
                "{{ name: \"{}\", value: {}, children: [",
                item.borrow().label,
                item.borrow().duration
            ))?;
        }

        for v in item.borrow().children.values() {
            self.write_stack_item(v)?;
            self.writable.write_fmt(format_args!(","))?;
        }

        if !self.is_root(item) {
            self.writable.write_fmt(format_args!("] }}"))?;
        }

        Ok(())
    }

    fn write_header(&mut self) -> std::io::Result<()> {
        self.writable.write_fmt(format_args!(
            r#"
<!doctype html>
<html>
    <head>
        <style>
            html, body {{
                width: 100%;
                height: 100%;
                margin: 0;
                padding: 0;
            }}
            {}
        </style>
        <script>
            {}
            {}
            {}
        </script>
    </head>
    <body>
        <script>
            var width = document.body.offsetWidth;
            var height = document.body.offsetHeight - 100;
            var flamegraph =
                d3.flameGraph()
                  .width(width)
                  .height(height)
                  .tooltip(false)
                  .sort(function(a, b){{
                    if (a.start < b.start) {{
                        return -1;
                    }} else if (a.start > b.start) {{
                        return 1;
                    }} else {{
                        return 0;
                    }}
                  }});
            d3.select("body").datum({{ children: ["#,
            include_str!("../../resources/flameGraph.css"),
            include_str!("../../resources/d3.js"),
            include_str!("../../resources/d3-tip.js"),
            include_str!("../../resources/flameGraph.js")
        ))
    }

    fn write_footer(&mut self) -> std::io::Result<()> {
        self.writable.write_fmt(format_args!(
            r#"]}}).call(flamegraph);
         </script>
    </body>
</html>"#
        ))
    }
}
