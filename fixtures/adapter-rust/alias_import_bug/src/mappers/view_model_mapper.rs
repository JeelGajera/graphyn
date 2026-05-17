use crate::models::user_payload::UserPayload as ResponseModel;

pub struct ViewModelMapper;

impl ViewModelMapper {
    pub fn map(&self, data: &ResponseModel) {
        let _ = &data.user_id;
        let _ = &data.timestamp;
        let _ = &data.status;
    }
}
