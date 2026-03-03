use crate::errors::Error;
use actix_web::web::Json;
use validator::{Validate, ValidationErrors};

/// Validate a struct using its `Validate` derive rules and return all errors.
#[must_use = "validation result must be checked"]
pub fn validate<T>(params: &Json<T>) -> Result<(), Error>
where
    T: Validate,
{
    match params.validate() {
        Ok(()) => Ok(()),
        Err(error) => Err(Error::Validation(collect_errors(error).join(";"))),
    }
}

fn collect_errors(error: ValidationErrors) -> Vec<String> {
    error
        .field_errors()
        .into_iter()
        .flat_map(|(field, errors)| {
            let default_error = format!("{} is required", field);
            errors.iter().map(move |e| {
                e.message
                    .as_ref()
                    .unwrap_or(&std::borrow::Cow::Owned(default_error.clone()))
                    .to_string()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CreateRoleEntry, CreateTeamEntry, CreateUserEntry};

    #[test]
    fn validate_accepts_valid_user() {
        let body = Json(CreateUserEntry {
            firstname: "Alice".to_string(),
            lastname: "Smith".to_string(),
            email: "alice@example.com".to_string(),
            password: "securepass".to_string(),
        });
        assert!(validate(&body).is_ok());
    }

    #[test]
    fn validate_rejects_short_firstname() {
        let body = Json(CreateUserEntry {
            firstname: "A".to_string(),
            lastname: "Smith".to_string(),
            email: "a@example.com".to_string(),
            password: "securepass".to_string(),
        });
        let err = validate(&body).unwrap_err();
        assert!(err.to_string().contains("firstname"));
    }

    #[test]
    fn validate_rejects_invalid_email() {
        let body = Json(CreateUserEntry {
            firstname: "Alice".to_string(),
            lastname: "Smith".to_string(),
            email: "not-an-email".to_string(),
            password: "securepass".to_string(),
        });
        assert!(validate(&body).is_err());
    }

    #[test]
    fn validate_rejects_short_password() {
        let body = Json(CreateUserEntry {
            firstname: "Alice".to_string(),
            lastname: "Smith".to_string(),
            email: "alice@example.com".to_string(),
            password: "short".to_string(),
        });
        let err = validate(&body).unwrap_err();
        assert!(err.to_string().contains("password"));
    }

    #[test]
    fn validate_reports_multiple_errors() {
        let body = Json(CreateUserEntry {
            firstname: "A".to_string(),
            lastname: "B".to_string(),
            email: "bad".to_string(),
            password: "short".to_string(),
        });
        let err = validate(&body).unwrap_err();
        let msg = err.to_string();
        // Multiple errors are joined with ";"
        assert!(msg.contains(';'), "expected multiple errors separated by ;");
    }

    #[test]
    fn validate_accepts_valid_team() {
        let body = Json(CreateTeamEntry {
            tname: "Engineering".to_string(),
            descr: Some("The eng team".to_string()),
        });
        assert!(validate(&body).is_ok());
    }

    #[test]
    fn validate_rejects_empty_team_name() {
        let body = Json(CreateTeamEntry {
            tname: "".to_string(),
            descr: None,
        });
        assert!(validate(&body).is_err());
    }

    #[test]
    fn validate_accepts_valid_role() {
        let body = Json(CreateRoleEntry {
            title: "Admin".to_string(),
        });
        assert!(validate(&body).is_ok());
    }

    #[test]
    fn validate_rejects_empty_role_title() {
        let body = Json(CreateRoleEntry {
            title: "".to_string(),
        });
        assert!(validate(&body).is_err());
    }
}
