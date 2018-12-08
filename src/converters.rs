use crate::LabelGetter;

mod debug_converter;
pub use self::debug_converter::DebugConverterFactory;

mod flamegraph_converter;
pub use self::flamegraph_converter::FlamegraphConverterFactory;

mod chrome_tracing_converter;
pub use self::chrome_tracing_converter::ChromeTracingConverterFactory;

pub trait Converter {
    fn process_event(
        &mut self,
        event: &hawktracer_parser::Event,
        reg: &hawktracer_parser::EventKlassRegistry,
    ) -> Result<(), Box<std::error::Error>>;

    fn close_converter(&mut self) {}
}

pub trait ConverterFactory {
    fn construct(&self, writable: Box<std::io::Write>, label_getter: LabelGetter)
        -> Box<Converter>;
    fn get_name(&self) -> &str;
}
