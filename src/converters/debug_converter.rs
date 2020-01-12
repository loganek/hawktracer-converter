use crate::converters::Converter;
use crate::ConverterFactory;
use crate::LabelGetter;

struct DebugConverter {
    writable: Box<dyn std::io::Write>,
    label_getter: LabelGetter,
    format_json: bool,
}

impl Converter for DebugConverter {
    fn process_event(
        &mut self,
        event: &hawktracer_parser::Event,
        reg: &hawktracer_parser::EventKlassRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.format_event(event, reg)?;
        Ok(())
    }
}

impl DebugConverter {
    pub fn new(
        writable: Box<dyn std::io::Write>,
        label_getter: LabelGetter,
        format_json: bool,
    ) -> DebugConverter {
        DebugConverter {
            writable,
            label_getter,
            format_json,
        }
    }

    fn write_value_pair(
        &mut self,
        key: &str,
        value: &hawktracer_parser::Value,
        map: Option<String>,
    ) -> std::io::Result<()> {
        self.writable.write_fmt(format_args!("    \"{}\": ", key))?;

        self.writable
            .write_fmt(format_args!("{{\n        \"value\": {},\n", value))?;
        let debug_value: String = (format!("{:?}", value)).replace("\"", "\\\"");
        if let Some(mapping) = map {
            self.writable
                .write_fmt(format_args!("        \"maps_to\": \"{}\",\n", mapping))?;
        }
        self.writable.write_fmt(format_args!(
            "        \"debug_value\": \"{}\"\n    }}",
            debug_value
        ))
    }

    fn get_klass_name(
        &self,
        event: &hawktracer_parser::Event,
        reg: &hawktracer_parser::EventKlassRegistry,
    ) -> String {
        match reg.get_klass_by_id(event.get_klass_id()) {
            Some(c) => c.get_name().to_string(),
            _ => format!("<unknown type, id: {}>", event.get_klass_id()),
        }
    }

    fn get_mapping(
        &mut self,
        event: &hawktracer_parser::Event,
        label_field: &String,
        value: &hawktracer_parser::Value,
    ) -> Option<String> {
        return match self.label_getter.get_label(event) {
            Some((field, label)) if field == label_field => {
                match value {
                    hawktracer_parser::Value::Str(_) => None,
                    _ => Some(label.clone())
                }
            }
            _ => None,
        };
    }

    fn format_event_json(
        &mut self,
        event: &hawktracer_parser::Event,
        reg: &hawktracer_parser::EventKlassRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.writable.write_all(b"{\n")?;
        let klass_name = self.get_klass_name(event, reg);
        self.writable.write_fmt(format_args!(
            "    \"meta_klass_name\": \"{}\",\n",
            &klass_name
        ))?;

        let mut first = true;
        for value in event.get_all_values() {
            if first {
                first = false;
            } else {
                self.writable.write_all(b",\n")?;
            }
            let mapping = self.get_mapping(event, value.0, &value.1);
            self.write_value_pair(value.0, &value.1, mapping)?;
        }
        self.writable.write_all(b"\n},\n")?;
        Ok(())
    }

    fn format_event_human(
        &mut self,
        event: &hawktracer_parser::Event,
        reg: &hawktracer_parser::EventKlassRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let klass_name = self.get_klass_name(event, reg);

        self.writable
            .write_fmt(format_args!("{} {{\n", klass_name))?;
        for value in event.get_all_values() {
            self.writable
                .write_fmt(format_args!("    \"{}\": {:?}", value.0, value.1))?;
            match self.get_mapping(event, value.0, &value.1) {
                Some(label) => {
                    self.writable
                        .write_fmt(format_args!(" <maps to {:?}>", label))?;
                }
                _ => {}
            };
            self.writable.write_all(b"\n")?;
        }
        self.writable.write_all(b"}\n")?;
        Ok(())
    }

    fn format_event(
        &mut self,
        event: &hawktracer_parser::Event,
        reg: &hawktracer_parser::EventKlassRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.format_json {
            self.format_event_json(event, reg)
        } else {
            self.format_event_human(event, reg)
        }
    }
}

pub struct DebugConverterFactory {}

impl ConverterFactory for DebugConverterFactory {
    fn construct(
        &self,
        writable: Box<dyn std::io::Write>,
        label_getter: LabelGetter,
    ) -> Box<dyn Converter> {
        Box::new(DebugConverter::new(writable, label_getter, false))
    }

    fn get_name(&self) -> &str {
        "debug"
    }
}

pub struct JSONDebugConverterFactory {}

impl ConverterFactory for JSONDebugConverterFactory {
    fn construct(
        &self,
        writable: Box<dyn std::io::Write>,
        label_getter: LabelGetter,
    ) -> Box<dyn Converter> {
        Box::new(DebugConverter::new(writable, label_getter, true))
    }

    fn get_name(&self) -> &str {
        "json_debug"
    }
}
