use crate::schema::DocumentTrait;
use crate::schema::Term;
use crate::Opstamp;

/// Timestamped Delete operation.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct DeleteOperation {
    pub opstamp: Opstamp,
    pub term: Term,
}

/// Timestamped Add operation.
#[derive(Eq, PartialEq, Debug)]
pub struct AddOperation<D: DocumentTrait> {
    pub opstamp: Opstamp,
    pub document: D,
}

/// UserOperation is an enum type that encapsulates other operation types.
#[derive(Eq, PartialEq, Debug)]
pub enum UserOperation<D: DocumentTrait> {
    /// Add operation
    Add(D),
    /// Delete operation
    Delete(Term),
}
