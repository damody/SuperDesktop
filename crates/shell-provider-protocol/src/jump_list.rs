use serde::{Deserialize, Serialize};

use crate::{CommandDescriptor, Validate, ValidationError, validate_command_tree};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JumpListRequest {
    pub application_id: String,
}

impl Validate for JumpListRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.application_id.trim().is_empty() {
            return Err(ValidationError::Empty("jump_list.application_id"));
        }
        if self.application_id.len() > crate::MAX_TEXT_BYTES {
            return Err(ValidationError::TextTooLong("jump_list.application_id"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct JumpListResponse {
    pub recent: Vec<CommandDescriptor>,
    pub frequent: Vec<CommandDescriptor>,
    pub tasks: Vec<CommandDescriptor>,
}

impl Validate for JumpListResponse {
    fn validate(&self) -> Result<(), ValidationError> {
        validate_command_tree(&self.recent)?;
        validate_command_tree(&self.frequent)?;
        validate_command_tree(&self.tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_rejects_empty_application_identity() {
        assert!(
            JumpListRequest {
                application_id: String::new()
            }
            .validate()
            .is_err()
        );
    }
}
