use crate::converters::Converter;
use crate::ConverterFactory;
use crate::LabelGetter;

struct DebugConverter {
    writable: Box<dyn std::io::Write>,
    label_getter: LabelGetter,
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
    pub fn new(writable: Box<dyn std::io::Write>, label_getter: LabelGetter) -> DebugConverter {
        DebugConverter {
            writable,
            label_getter,
        }
    }

    fn format_event(
        &mut self,
        event: &hawktracer_parser::Event,
        reg: &hawktracer_parser::EventKlassRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let type_name = match reg.get_klass_by_id(event.get_klass_id()) {
            Some(c) => c.get_name().to_string(),
            _ => format!("<unknown type, id: {}>", event.get_klass_id()),
        };
        let label_mapping = self.label_getter.get_label(event);

        self.writable
            .write_fmt(format_args!("{} {{\n", type_name))?;
        for value in event.get_all_values() {
            self.writable
                .write_fmt(format_args!("    \"{}\": {:?}", value.0, value.1))?;
            match label_mapping {
                Some((field, label)) if field == value.0 => {
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
}

pub struct DebugConverterFactory {}

impl ConverterFactory for DebugConverterFactory {
    fn construct(
        &self,
        writable: Box<dyn std::io::Write>,
        label_getter: LabelGetter,
    ) -> Box<dyn Converter> {
        Box::new(DebugConverter::new(writable, label_getter))
    }

    fn get_name(&self) -> &str {
        "debug"
    }
}
