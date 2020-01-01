use crate::converters::Converter;
use crate::ConverterFactory;
use crate::LabelGetter;

use hawktracer_parser::{Event, EventKlassRegistry, Value};

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

    fn ns_to_us(nano_secs: u64) -> u64 {
        nano_secs / 1000 + (nano_secs % 1000) / 500
    }

    fn format_free_arg(field_name: &str, value: &Value) -> String {
        format!("{:?}: {}", field_name, value)
    }

    fn get_free_args(&mut self) -> String {
        let mut args_str = String::new();

        if let Ok(klass_id) = self.event.get_value_u32("type") {
            if let Some(klass) = self.reg.get_klass_by_id(klass_id) {
                self.used_fields.insert("type");
                args_str.push_str(&EventWriter::format_free_arg(
                    "type",
                    &Value::Str(klass.get_name().clone()),
                ));
            }
        }

        for (field_name, value) in self.event.get_all_values() {
            if self.used_fields.contains(&field_name[..]) {
                continue;
            }

            if !args_str.is_empty() {
                args_str.push(',');
            }

            args_str.push_str(&EventWriter::format_free_arg(field_name, value));
        }
        args_str
    }

    pub fn write_event(
        &mut self,
        writable: &mut dyn std::io::Write,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = EventWriter::ns_to_us(self.event.get_value_u64("timestamp")?);

        let duration = match self.event.get_value_u64("duration") {
            Ok(duration) => EventWriter::ns_to_us(duration),
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestWritable {
        buffer: Vec<u8>,
    }

    impl TestWritable {
        pub fn new() -> TestWritable {
            TestWritable { buffer: Vec::new() }
        }

        pub fn get_buffer(&self) -> &Vec<u8> {
            &self.buffer
        }
    }

    impl std::io::Write for TestWritable {
        fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
            self.buffer.extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
            Ok(())
        }
    }

    #[test]
    fn nanosecond_to_microsecond_test() {
        assert_eq!(EventWriter::ns_to_us(1234), 1);
        assert_eq!(EventWriter::ns_to_us(89999), 90);
        assert_eq!(EventWriter::ns_to_us(60000), 60);
        assert_eq!(EventWriter::ns_to_us(32500), 33);
        assert_eq!(EventWriter::ns_to_us(5), 0);
    }

    #[test]
    fn format_free_arg_should_format_argument_correctly() {
        assert_eq!(
            EventWriter::format_free_arg("field", &Value::U16(12)),
            "\"field\": 12"
        );
        assert_eq!(
            EventWriter::format_free_arg("field", &Value::Str("value".to_owned())),
            "\"field\": \"value\""
        );
    }

    #[test]
    fn write_event_should_generate_valid_json_object() {
        let mut writable = TestWritable::new();
        let mut values = std::collections::HashMap::new();
        values.insert("timestamp".to_owned(), Value::U64(5999));
        values.insert("duration".to_owned(), Value::U64(12000));
        values.insert("field1".to_owned(), Value::I32(-45));
        values.insert("thread_id".to_owned(), Value::U32(7));
        EventWriter::new(
            &Event::new(99, values),
            &EventKlassRegistry::new(),
            "label",
            "field",
        )
        .write_event(&mut writable)
        .unwrap();

        let data = std::str::from_utf8(writable.get_buffer()).unwrap();

        assert_eq!("{\"name\":\"label\",\"ph\":\"X\",\"ts\":6,\"dur\":12,\"pid\":0,\"tid\":7, \"args\": { \"field1\": -45 } },", data);
    }
}
