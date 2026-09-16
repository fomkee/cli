use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::CliError;

macro_rules! identifier {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(Uuid);

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                Display::fmt(&self.0, formatter)
            }
        }

        impl FromStr for $name {
            type Err = CliError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(value)
                    .map(Self)
                    .map_err(|_| CliError::InvalidInput(format!("{} must be a UUID", $label)))
            }
        }
    };
}

identifier!(WorkspaceId, "workspace ID");
identifier!(MonitorId, "monitor ID");
identifier!(DestinationId, "destination ID");
identifier!(AssignmentId, "assignment ID");
identifier!(IncidentId, "incident ID");
identifier!(IncidentEventId, "incident event ID");
identifier!(MaintenanceId, "maintenance ID");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_non_uuid_monitor_identifier() {
        // Act
        let result = "not-a-monitor-id".parse::<MonitorId>();

        // Assert
        assert!(matches!(result, Err(CliError::InvalidInput(_))));
    }

    #[test]
    fn test_preserves_workspace_identifier_display() -> Result<(), CliError> {
        // Arrange
        let value = "00000000-0000-0000-0000-000000000001";

        // Act
        let workspace = value.parse::<WorkspaceId>()?;

        // Assert
        assert_eq!(workspace.to_string(), value);
        Ok(())
    }
}
