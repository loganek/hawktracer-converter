use crate::converters::Converter;
use crate::converters::ConverterFactory;
use crate::LabelGetter;

pub struct ConverterManager {
    factories: std::vec::Vec<Box<ConverterFactory>>,
}

impl Default for ConverterManager {
    fn default() -> ConverterManager {
        ConverterManager::new()
    }
}

impl ConverterManager {
    pub fn new() -> ConverterManager {
        let mut manager = ConverterManager {
            factories: std::vec::Vec::<Box<ConverterFactory>>::new(),
        };
        manager.load_embedded_converters();
        manager
    }

    fn load_embedded_converters(&mut self) {
        self.register_static_factory(crate::converters::DebugConverterFactory {});
        self.register_static_factory(crate::converters::ChromeTracingConverterFactory {});
        self.register_static_factory(crate::converters::FlamegraphConverterFactory {});
    }

    pub fn create_converter(
        &self,
        name: &str,
        writable: Box<std::io::Write>,
        label_getter: LabelGetter,
    ) -> Option<Box<Converter>> {
        for factory in &self.factories {
            if name == factory.get_name() {
                return Some(factory.construct(writable, label_getter));
            }
        }
        None
    }

    pub fn get_converters(&self) -> std::vec::Vec<&str> {
        let mut v = vec![];

        for factory in &self.factories {
            v.push(factory.get_name())
        }

        v
    }

    pub fn register_static_factory<T: ConverterFactory + 'static>(&mut self, format: T) {
        self.factories.push(Box::new(format));
    }
}
