use DocId;

use super::fieldnorm_to_id;
use super::FieldNormsSerializer;
use schema::Field;
use schema::Schema;
use std::io;

/// The `FieldNormsWriter` is in charge of tracking the fieldnorm byte
/// of each document for each field with field norms.
///
/// `FieldNormsWriter` stores a Vec<u8> for each tracked field, using a
/// byte per document per field.
pub struct FieldNormsWriter {
    fields: Vec<Field>,
    fieldnorms_buffer: Vec<Vec<u8>>,
}

impl FieldNormsWriter {
    /// Returns the fields that should have field norms computed
    /// according to the given schema.
    pub(crate) fn fields_with_fieldnorm(schema: &Schema) -> Vec<Field> {
        schema
            .fields()
            .iter()
            .enumerate()
            .filter(|&(_, field_entry)| field_entry.is_indexed())
            .map(|(field, _)| Field(field as u32))
            .collect::<Vec<Field>>()
    }

    /// Initialize with state for tracking the field norm fields
    /// specified in the schema.
    pub fn for_schema(schema: &Schema) -> FieldNormsWriter {
        let fields = FieldNormsWriter::fields_with_fieldnorm(schema);
        let max_field = fields
            .iter()
            .map(|field| field.0)
            .max()
            .map(|max_field_id| max_field_id as usize + 1)
            .unwrap_or(0);
        FieldNormsWriter {
            fields,
            fieldnorms_buffer: (0..max_field).map(|_| Vec::new()).collect::<Vec<_>>(),
        }
    }

    /// Ensure that all documents in 0..max_doc have a byte associated with them
    /// in each of the fieldnorm vectors.
    ///
    /// Will extend with 0-bytes for documents that have not been seen.
    pub fn fill_up_to_max_doc(&mut self, max_doc: DocId) {
        for &field in self.fields.iter() {
            self.fieldnorms_buffer[field.0 as usize].resize(max_doc as usize, 0u8);
        }
    }

    /// Set the fieldnorm byte for the given document for the given field.
    ///
    /// Will internally convert the u32 `fieldnorm` value to the appropriate byte
    /// to approximate the field norm in less space.
    ///
    /// * doc       - the document id
    /// * field     - the field being set
    /// * fieldnorm - the number of terms present in document `doc` in field `field`
    pub fn record(&mut self, doc: DocId, field: Field, fieldnorm: u32) {
        let fieldnorm_buffer: &mut Vec<u8> = &mut self.fieldnorms_buffer[field.0 as usize];
        assert!(
            fieldnorm_buffer.len() <= doc as usize,
            "Cannot register a given fieldnorm twice"
        );
        // we fill intermediary `DocId` as  having a fieldnorm of 0.
        fieldnorm_buffer.resize(doc as usize + 1, 0u8);
        fieldnorm_buffer[doc as usize] = fieldnorm_to_id(fieldnorm);
    }

    /// Serialize the seen fieldnorm values to the serializer for all fields.
    pub fn serialize(&self, fieldnorms_serializer: &mut FieldNormsSerializer) -> io::Result<()> {
        for &field in self.fields.iter() {
            let fieldnorm_values: &[u8] = &self.fieldnorms_buffer[field.0 as usize][..];
            fieldnorms_serializer.serialize_field(field, fieldnorm_values)?;
        }
        Ok(())
    }
}
