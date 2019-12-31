use crate::converters::Converter;
use crate::ConverterFactory;
use crate::LabelGetter;

use hawktracer_parser::{Event, EventKlassRegistry};

struct ChromeTracingConverter {
    writable: Box<dyn std::io::Write>,
    header_written: bool,
    label_getter: LabelGetter,
}

#[derive(Debug)]
pub enum EventProcessingErrorKind {
    MissingLabelField,
    InvalidType,
}

#[derive(Debug)]
pub struct EventProcessingError {
    kind: EventProcessingErrorKind,
    info: String,
}

impl EventProcessingError {
    pub fn new(kind: EventProcessingErrorKind, info: String) -> EventProcessingError {
        EventProcessingError { kind, info }
    }
}

impl std::fmt::Display for EventProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Invalid label: {:?} {}", self.kind, self.info)
    }
}

impl std::error::Error for EventProcessingError {}

impl Converter for ChromeTracingConverter {
    fn process_event(
        &mut self,
        event: &Event,
        reg: &EventKlassRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.header_written {
            self.writable.write_all(b"[")?;
            self.header_written = true;
        }

        let label_mapping = self.label_getter.get_label(event);
        let label_field = match label_mapping {
            Some(label_mapping) => label_mapping.0,
            None => {
                return Err(Box::new(EventProcessingError::new(
                    EventProcessingErrorKind::MissingLabelField,
                    String::new(),
                )));
            }
        };

        let label = match label_mapping {
            Some(label_mapping) => label_mapping.1,
            None => {
                return Err(Box::new(EventProcessingError::new(
                    EventProcessingErrorKind::InvalidType,
                    label_field.clone(),
                )));
            }
        };

        EventWriter::new(event, reg, &label, &label_field).write_event(&mut self.writable)
    }
}

impl ChromeTracingConverter {
    pub fn new(
        writable: Box<dyn std::io::Write>,
        label_getter: LabelGetter,
    ) -> ChromeTracingConverter {
        ChromeTracingConverter {
            writable,
            header_written: false,
            label_getter,
        }
    }
}

struct EventWriter<'a> {
    event: &'a Event,
    used_fields: std::collections::HashSet<&'a str>,
    reg: &'a EventKlassRegistry,
    label: &'a str,
}

impl<'a> EventWriter<'a> {
    const INVALID_THREAD_ID: u32 = 99;

    pub fn new(
        event: &'a Event,
        reg: &'a EventKlassRegistry,
        label: &'a str,
        label_field: &'a str,
    ) -> EventWriter<'a> {
        let mut used_fields = std::collections::HashSet::<&str>::new();
        used_fields.insert("timestamp");
        used_fields.insert("duration");
        used_fields.insert("thread_id");
        used_fields.insert(label_field);

        EventWriter {
            event,
            used_fields,
            reg,
            label,
        }
    }

    fn ns_to_ms(&self, nano_secs: u64) -> u64 {
        nano_secs / 1000
    }

    fn format_free_arg<T: std::fmt::Display>(&self, field_name: &str, value: &T) -> String {
        format!("\"{}\": {}", field_name, value)
    }

    fn get_free_args(&mut self) -> String {
        let mut args_str = String::new();

        if let Ok(klass_id) = self.event.get_value_u32("type") {
            if let Some(klass) = self.reg.get_klass_by_id(klass_id) {
                self.used_fields.insert("type");
                let klass_str = format!("\"{}\"", klass.get_name());
                args_str.push_str(&self.format_free_arg("type", &klass_str));
            }
        }

        for (field_name, value) in self.event.get_all_values() {
            if self.used_fields.contains(&field_name[..]) {
                continue;
            }

            if !args_str.is_empty() {
                args_str.push(',');
            }

            args_str.push_str(&self.format_free_arg(field_name, value));
        }
        args_str
    }

    pub fn write_event(
        &mut self,
        writable: &mut dyn std::io::Write,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = self.ns_to_ms(self.event.get_value_u64("timestamp")?);

        let duration = match self.event.get_value_u64("duration") {
            Ok(duration) => self.ns_to_ms(duration),
            Err(_) => 0,
        };
        let thread_id = match self.event.get_value_u32("thread_id") {
            Ok(thread_id) => thread_id,
            Err(_) => EventWriter::INVALID_THREAD_ID,
        };

        let free_args = self.get_free_args();

        writable.write_fmt(format_args!(
            r#"{{"name":"{}","ph":"X","ts":{},"dur":{},"pid":0,"tid":{}, "args": {{ {} }} }},"#,
            self.label, timestamp, duration, thread_id, free_args
        ))?;

        Ok(())
    }
}

pub struct ChromeTracingConverterFactory {}

impl ConverterFactory for ChromeTracingConverterFactory {
    fn construct(
        &self,
        writable: Box<dyn std::io::Write>,
        label_getter: LabelGetter,
    ) -> Box<dyn Converter> {
        Box::new(ChromeTracingConverter::new(writable, label_getter))
    }

    fn get_name(&self) -> &str {
        "chrome-tracing"
    }
}
