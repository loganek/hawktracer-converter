use crate::converters::Converter;
use crate::converters::ConverterFactory;
use crate::LabelGetter;

pub struct ConverterManager {
    factories: std::vec::Vec<Box<dyn ConverterFactory>>,
}

impl Default for ConverterManager {
    fn default() -> ConverterManager {
        ConverterManager::new()
    }
}

impl ConverterManager {
    pub fn new() -> ConverterManager {
        let mut manager = ConverterManager {
            factories: std::vec::Vec::<Box<dyn ConverterFactory>>::new(),
        };
        manager.load_embedded_converters();
        manager
    }

    fn load_embedded_converters(&mut self) {
        self.register_static_factory(crate::converters::ChromeTracingConverterFactory {});
        self.register_static_factory(crate::converters::DebugConverterFactory {});
        self.register_static_factory(crate::converters::JSONDebugConverterFactory {});
        self.register_static_factory(crate::converters::FlamegraphConverterFactory {});
    }

    pub fn create_converter(
        &self,
        name: &str,
        writable: Box<dyn std::io::Write>,
        label_getter: LabelGetter,
    ) -> Option<Box<dyn Converter>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LabelMap;
    use std::iter::Iterator;

    struct DummyConverter {}

    impl Converter for DummyConverter {
        fn process_event(
            &mut self,
            _event: &hawktracer_parser::Event,
            _reg: &hawktracer_parser::EventKlassRegistry,
        ) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }

    impl DummyConverter {
        pub fn new() -> DummyConverter {
            DummyConverter {}
        }
    }

    pub struct DummyConverterFactory {}

    impl ConverterFactory for DummyConverterFactory {
        fn construct(
            &self,
            _writable: Box<dyn std::io::Write>,
            _label_getter: LabelGetter,
        ) -> Box<dyn Converter> {
            Box::new(DummyConverter::new())
        }

        fn get_name(&self) -> &str {
            "dummy"
        }
    }

    #[test]
    fn create_factory_should_return_none_for_non_existing_converter() {
        let manager: ConverterManager = Default::default();

        let label_getter = LabelGetter::new(LabelMap::new(), vec![]);
        assert!(manager
            .create_converter(
                "invalid-converter",
                Box::new(std::io::stdout()),
                label_getter
            )
            .is_none());
    }

    #[test]
    fn get_converters_should_return_newly_registered_converter() {
        let mut manager: ConverterManager = Default::default();
        manager.register_static_factory(DummyConverterFactory {});

        assert!(manager
            .get_converters()
            .iter()
            .find(|&&x| x == "dummy")
            .is_some());
    }

    #[test]
    fn create_converter_should_return_newly_registered_converter() {
        let mut manager: ConverterManager = Default::default();
        manager.register_static_factory(DummyConverterFactory {});

        let label_getter = LabelGetter::new(LabelMap::new(), vec![]);
        let mut converter = manager
            .create_converter("dummy", Box::new(std::io::stdout()), label_getter)
            .unwrap();

        assert!(converter
            .process_event(
                &hawktracer_parser::Event::new(1, std::collections::HashMap::new()),
                &hawktracer_parser::EventKlassRegistry::new()
            )
            .is_ok());
    }
}
