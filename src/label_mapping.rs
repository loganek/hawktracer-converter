use hawktracer_parser::{CoreEventKlassId, Event, Value};
use std::io::BufRead;

// TODO: SUPPORT MAPPING EVENT

#[derive(Default)]
pub struct LabelMap {
    mapping: std::collections::HashMap<u64, String>,
}

impl LabelMap {
    pub fn new() -> LabelMap {
        LabelMap {
            mapping: std::collections::HashMap::<u64, String>::new(),
        }
    }

    pub fn load_from_file(&mut self, path: &str) -> std::io::Result<()> {
        let f = std::fs::File::open(path)?;
        let file = std::io::BufReader::new(&f);
        for (i, line) in file.lines().enumerate() {
            let line = line?;
            let data: Vec<&str> = line.split(' ').collect();
            if data.len() != 3 {
                eprintln!(
                    "invalid mapping in line {}. Expected 3 arguments, was {}",
                    i,
                    data.len()
                );
                continue;
            }

            let id = match data[2].parse::<u64>() {
                Err(e) => {
                    eprintln!(
                        "Can not parse identifier '{}' in line {}. Error message: {}",
                        data[2], i, e
                    );
                    continue;
                }
                Ok(id) => id,
            };

            let label = data[1].to_string();
            self.mapping.insert(id, label); // TODO support data[0] (category)?
        }

        Ok(())
    }

    pub fn get_label(&mut self, id: u64) -> &String {
        self.mapping.entry(id).or_insert_with(|| {
            eprintln!("Label for ID {} does not exist in the mapping", id);
            id.to_string()
        })
    }

    pub fn add_mapping(&mut self, id: u64, label: &str) {
        self.mapping.insert(id, label.to_owned());
    }
}

pub struct LabelGetter {
    label_map: LabelMap,
    label_fields: std::vec::Vec<String>,
    mapping_event_id: Option<u32>,
}

impl LabelGetter {
    pub fn new(label_map: LabelMap, label_fields: std::vec::Vec<String>) -> LabelGetter {
        LabelGetter {
            label_map,
            label_fields,
            mapping_event_id: None,
        }
    }

    // TODO TEST ME PLEASE!
    fn update_mapping_event_info(&mut self, event: &Event) {
        if self.mapping_event_id.is_none()
            && event.get_klass_id() == CoreEventKlassId::KlassInfo as u32
        {
            let klass_name = event.get_value_string("event_klass_name");
            if klass_name.is_ok() && klass_name.unwrap() == "HT_StringMappingEvent" {
                // TODO We should have KlassInfo event wrapper with get_klass_name method in parser
                self.mapping_event_id = event.get_value_u32("info_klass_id").ok();
            }
        } else if self.mapping_event_id.is_some()
            && event.get_klass_id() == self.mapping_event_id.unwrap()
        {
            let id = event.get_value_u64("identifier");
            let label = event.get_value_string("label");
            if label.is_ok() && id.is_ok() {
                self.label_map.add_mapping(id.unwrap(), label.unwrap());
            }
        }
    }

    pub fn get_label<'a>(
        &'a mut self,
        event: &'a Event,
    ) -> (Option<&'a String>, Option<&'a String>) {
        self.update_mapping_event_info(event);

        for label_field in &self.label_fields {
            if let Some(value) = event.get_raw_value(label_field) {
                match value {
                    Value::U64(value) => {
                        return (Some(label_field), Some(self.label_map.get_label(*value)));
                    }
                    Value::Str(value) => return (Some(label_field), Some(value)),
                    _ => (),
                }
            }
        }
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_file_valid_file_should_load_mapping() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/test/label_map_valid.txt");
        let mut map = LabelMap::new();

        map.load_from_file(&path.to_str().unwrap());

        assert_eq!(map.get_label(1), "label1");
        assert_eq!(map.get_label(2), "label2");
        assert_eq!(map.get_label(3), "label3");
    }

    #[test]
    fn load_from_file_corrupted_file_should_ignore_invalid_values() {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/test/label_map_invalid.txt");
        let mut map = LabelMap::new();

        map.load_from_file(&path.to_str().unwrap());

        assert_eq!(map.get_label(8), "valid_label");
    }

    #[test]
    fn load_from_file_should_fail_if_file_does_not_exist() {
        let mut map = LabelMap::new();

        assert!(map.load_from_file("not_existing_file").is_err());
    }

    #[test]
    fn map_should_return_number_if_mapping_does_not_exist() {
        let mut map = LabelMap::new();

        assert_eq!(map.get_label(4), "4");
    }

    #[test]
    fn map_should_return_label_if_mapping_exist() {
        let mut map = LabelMap::new();
        let label = "test";
        map.mapping.insert(4, label.to_owned());

        assert_eq!(map.get_label(4), label);
    }

    fn make_event(field_name: &str, value: Value) -> Event {
        let mut values = std::collections::HashMap::<String, Value>::new();
        values.insert(field_name.to_owned(), value);
        Event::new(1, values)
    }

    #[test]
    fn getter_should_return_value_if_value_exists() {
        let mut getter = LabelGetter::new(LabelMap::new(), vec!["name".to_owned()]);
        let event = make_event("name", Value::Str("test1".to_owned()));

        let (field, value) = getter.get_label(&event);

        assert_eq!(field.unwrap(), "name");
        assert_eq!(value.unwrap(), "test1");
    }

    #[test]
    fn getter_should_not_return_value_if_field_does_not_exist() {
        let mut getter = LabelGetter::new(LabelMap::new(), vec!["unknown".to_owned()]);
        let event = make_event("name", Value::Str("test1".to_owned()));

        let (field, value) = getter.get_label(&event);

        assert!(field.is_none());
        assert!(value.is_none());
    }

    #[test]
    fn getter_should_not_return_value_if_value_is_invalid() {
        let mut getter = LabelGetter::new(LabelMap::new(), vec!["name".to_owned()]);
        let event = make_event(
            "name",
            Value::Struct(Event::new(
                1,
                std::collections::HashMap::<String, Value>::new(),
            )),
        );

        let (field, value) = getter.get_label(&event);

        assert!(field.is_none());
        assert!(value.is_none());
    }
}
