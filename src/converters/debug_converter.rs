use crate::converters::Converter;
use crate::ConverterFactory;
use crate::LabelGetter;

struct DebugConverter {
    writable: Box<dyn std::io::Write>,
}

impl Converter for DebugConverter {
    fn process_event(
        &mut self,
        event: &hawktracer_parser::Event,
        _reg: &hawktracer_parser::EventKlassRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.writable.write_fmt(format_args!("{:?}", &event))?;
        Ok(())
    }
}

impl DebugConverter {
    pub fn new(writable: Box<dyn std::io::Write>) -> DebugConverter {
        DebugConverter { writable }
    }
}

pub struct DebugConverterFactory {}

impl ConverterFactory for DebugConverterFactory {
    fn construct(
        &self,
        writable: Box<dyn std::io::Write>,
        _label_getter: LabelGetter,
    ) -> Box<dyn Converter> {
        Box::new(DebugConverter::new(writable))
    }

    fn get_name(&self) -> &str {
        "debug"
    }
}
