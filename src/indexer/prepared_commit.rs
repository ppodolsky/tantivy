use super::IndexWriter;
use crate::Opstamp;
use crate::schema::DocumentTrait;
use futures::executor::block_on;

/// A prepared commit
pub struct PreparedCommit<'a, D: 'static + DocumentTrait> {
    index_writer: &'a mut IndexWriter<D>,
    payload: Option<String>,
    opstamp: Opstamp,
}

impl<'a, D: 'static + DocumentTrait> PreparedCommit<'a, D> {
    pub(crate) fn new(index_writer: &'a mut IndexWriter<D>, opstamp: Opstamp) -> PreparedCommit<'_, D> {
        PreparedCommit {
            index_writer,
            payload: None,
            opstamp,
        }
    }

    pub fn opstamp(&self) -> Opstamp {
        self.opstamp
    }

    pub fn set_payload(&mut self, payload: &str) {
        self.payload = Some(payload.to_string())
    }

    pub fn abort(self) -> crate::Result<Opstamp> {
        self.index_writer.rollback()
    }

    pub fn commit(self) -> crate::Result<Opstamp> {
        info!("committing {}", self.opstamp);
        let _ = block_on(
            self.index_writer
                .segment_updater()
                .schedule_commit(self.opstamp, self.payload),
        );
        Ok(self.opstamp)
    }
}
