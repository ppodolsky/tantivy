use super::*;
use crate::common::BinarySerializable;
use crate::common::VInt;
use crate::tokenizer::PreTokenizedString;
use crate::DateTime;
use std::io::{self, Read, Write};
use std::mem;

/// Tantivy's Document is the object that can
/// be indexed and then searched for.
///
/// Documents are fundamentally a collection of unordered couple `(field, value)`.
/// In this list, one field may appear more than once.
///
///

pub trait DocumentTrait: BinarySerializable + Clone + Send {
    /// Creates a new, empty document object
    fn new() -> Self;

    /// Returns the number of `(field, value)` pairs.
    fn len(&self) -> usize;

    /// Returns true iff the document contains no fields.
    fn is_empty(&self) -> bool;

    /// Retain only the field that are matching the
    /// predicate given in argument.
    fn filter_fields<P: Fn(Field) -> bool>(&mut self, predicate: P);

    /// Adding a facet to the document.
    fn add_facet<F>(&mut self, field: Field, path: F) where Facet: From<F>;

    /// Add a text field.
    fn add_text<S: ToString>(&mut self, field: Field, text: S);

    /// Add a pre-tokenized text field.
    fn add_pre_tokenized_text(
        &mut self,
        field: Field,
        pre_tokenized_text: &PreTokenizedString,
    );

    /// Add a u64 field
    fn add_u64(&mut self, field: Field, value: u64);

    /// Add a i64 field
    fn add_i64(&mut self, field: Field, value: i64);

    /// Add a f64 field
    fn add_f64(&mut self, field: Field, value: f64);

    /// Add a date field
    fn add_date(&mut self, field: Field, value: &DateTime);

    /// Add a bytes field
    fn add_bytes<T: Into<Vec<u8>>>(&mut self, field: Field, value: T);

    /// Add a field value
    fn add(&mut self, field_value: FieldValue);

    /// field_values accessor
    fn field_values(&self) -> &[FieldValue];

    /// Sort and groups the field_values by field.
    ///
    /// The result of this method is not cached and is
    /// computed on the fly when this method is called.
    fn get_sorted_field_values(&self) -> Vec<(Field, Vec<&FieldValue>)>;

    /// Returns all of the `FieldValue`s associated the given field
    fn get_all(&self, field: Field) -> Vec<&Value>;

    /// Returns the first `FieldValue` associated the given field
    fn get_first(&self, field: Field) -> Option<&Value>;

    /// Prepares Document for being stored in the document store
    ///
    /// Method transforms PreTokenizedString values into String
    /// values.
    fn prepare_for_store(&mut self);
}

/// Documents are really just a list of couple `(field, value)`.
/// In this list, one field may appear more than once.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct Document {
    field_values: Vec<FieldValue>,
}

impl From<Vec<FieldValue>> for Document {
    fn from(field_values: Vec<FieldValue>) -> Self {
        Document { field_values }
    }
}

impl PartialEq for Document {
    fn eq(&self, other: &Document) -> bool {
        // super slow, but only here for tests
        let mut self_field_values: Vec<&_> = self.field_values.iter().collect();
        let mut other_field_values: Vec<&_> = other.field_values.iter().collect();
        self_field_values.sort();
        other_field_values.sort();
        self_field_values.eq(&other_field_values)
    }
}

impl Eq for Document {}

impl DocumentTrait for Document {
    fn new() -> Document {
        Document::default()
    }

    fn len(&self) -> usize {
        self.field_values.len()
    }

    fn is_empty(&self) -> bool {
        self.field_values.is_empty()
    }

    fn filter_fields<P: Fn(Field) -> bool>(&mut self, predicate: P) {
        self.field_values
            .retain(|field_value| predicate(field_value.field()));
    }

    fn add_facet<F>(&mut self, field: Field, path: F)
    where
        Facet: From<F>,
    {
        let facet = Facet::from(path);
        let value = Value::Facet(facet);
        self.add(FieldValue::new(field, value));
    }

    fn add_text<S: ToString>(&mut self, field: Field, text: S) {
        self.add(FieldValue::new(field, Value::Str(text.to_string())));
    }

    fn add_pre_tokenized_text(
        &mut self,
        field: Field,
        pre_tokenized_text: &PreTokenizedString,
    ) {
        let value = Value::PreTokStr(pre_tokenized_text.clone());
        self.add(FieldValue::new(field, value));
    }

    fn add_u64(&mut self, field: Field, value: u64) {
        self.add(FieldValue::new(field, Value::U64(value)));
    }

    fn add_i64(&mut self, field: Field, value: i64) {
        self.add(FieldValue::new(field, Value::I64(value)));
    }

    fn add_f64(&mut self, field: Field, value: f64) {
        self.add(FieldValue::new(field, Value::F64(value)));
    }

    fn add_date(&mut self, field: Field, value: &DateTime) {
        self.add(FieldValue::new(field, Value::Date(*value)));
    }

    fn add_bytes<T: Into<Vec<u8>>>(&mut self, field: Field, value: T) {
        self.add(FieldValue::new(field, Value::Bytes(value.into())))
    }

    fn add(&mut self, field_value: FieldValue) {
        self.field_values.push(field_value);
    }

    fn field_values(&self) -> &[FieldValue] {
        &self.field_values
    }

    fn get_sorted_field_values(&self) -> Vec<(Field, Vec<&FieldValue>)> {
        let mut field_values: Vec<&FieldValue> = self.field_values().iter().collect();
        field_values.sort_by_key(|field_value| field_value.field());

        let mut grouped_field_values = vec![];

        let mut current_field;
        let mut current_group;

        let mut field_values_it = field_values.into_iter();
        if let Some(field_value) = field_values_it.next() {
            current_field = field_value.field();
            current_group = vec![field_value]
        } else {
            return grouped_field_values;
        }

        for field_value in field_values_it {
            if field_value.field() == current_field {
                current_group.push(field_value);
            } else {
                grouped_field_values.push((
                    current_field,
                    mem::replace(&mut current_group, vec![field_value]),
                ));
                current_field = field_value.field();
            }
        }

        grouped_field_values.push((current_field, current_group));
        grouped_field_values
    }

    fn get_all(&self, field: Field) -> Vec<&Value> {
        self.field_values
            .iter()
            .filter(|field_value| field_value.field() == field)
            .map(FieldValue::value)
            .collect()
    }

    fn get_first(&self, field: Field) -> Option<&Value> {
        self.field_values
            .iter()
            .find(|field_value| field_value.field() == field)
            .map(FieldValue::value)
    }

    fn prepare_for_store(&mut self) {
        for field_value in &mut self.field_values {
            if let Value::PreTokStr(pre_tokenized_text) = field_value.value() {
                *field_value = FieldValue::new(
                    field_value.field(),
                    Value::Str(pre_tokenized_text.text.clone()), //< TODO somehow remove .clone()
                );
            }
        }
    }
}

impl BinarySerializable for Document {
    fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let field_values = self.field_values();
        VInt(field_values.len() as u64).serialize(writer)?;
        for field_value in field_values {
            field_value.serialize(writer)?;
        }
        Ok(())
    }

    fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self> {
        let num_field_values = VInt::deserialize(reader)?.val() as usize;
        let field_values = (0..num_field_values)
            .map(|_| FieldValue::deserialize(reader))
            .collect::<io::Result<Vec<FieldValue>>>()?;
        Ok(Document::from(field_values))
    }
}

#[cfg(test)]
mod tests {

    use crate::schema::*;
    use crate::tokenizer::{PreTokenizedString, Token};

    #[test]
    fn test_doc() {
        let mut schema_builder = Schema::builder();
        let text_field = schema_builder.add_text_field("title", TEXT);
        let mut doc = Document::default();
        doc.add_text(text_field, "My title");
        assert_eq!(doc.field_values().len(), 1);
    }

    #[test]
    fn test_prepare_for_store() {
        let mut schema_builder = Schema::builder();
        let text_field = schema_builder.add_text_field("title", TEXT);
        let mut doc = Document::default();

        let pre_tokenized_text = PreTokenizedString {
            text: String::from("A"),
            tokens: vec![Token {
                offset_from: 0,
                offset_to: 1,
                position: 0,
                text: String::from("A"),
                position_length: 1,
            }],
        };

        doc.add_pre_tokenized_text(text_field, &pre_tokenized_text);
        doc.add_text(text_field, "title");
        doc.prepare_for_store();

        assert_eq!(doc.field_values().len(), 2);

        match doc.field_values()[0].value() {
            Value::Str(ref text) => assert_eq!(text, "A"),
            _ => panic!("Incorrect variant of Value"),
        }

        match doc.field_values()[1].value() {
            Value::Str(ref text) => assert_eq!(text, "title"),
            _ => panic!("Incorrect variant of Value"),
        }
    }
}
