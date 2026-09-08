use super::*;
use crate::dto::Schedule;
use crate::dto::request::{ActiveRequest, CreateConfig};
use url::Url;

impl CreateCommand {
    pub fn needs_interval(&self) -> bool {
        match self {
            Self::Http(args) => args.common.file.is_none() && args.interval_secs.is_none(),
            Self::Function(args) => args.common.file.is_none() && args.interval_secs.is_none(),
            Self::Heartbeat(_) => false,
        }
    }

    pub fn into_input(self) -> Result<CreateInput, CliError> {
        match self {
            Self::Http(args) => args.into_input(),
            Self::Function(args) => args.into_input(),
            Self::Heartbeat(args) => args.into_input(),
        }
    }
}

impl CommonCreateArgs {
    fn file_input(&self, expected: MonitorKind) -> Result<Option<CreateInput>, CliError> {
        self.file
            .as_deref()
            .map(|path| {
                let input = CreateInput::try_from(read_json_input(path)?)?;
                if input.kind() != expected {
                    return Err(CliError::InvalidInput(format!(
                        "create {} requires the matching config_type",
                        expected.label()
                    )));
                }
                Ok(input)
            })
            .transpose()
    }

    fn name_for_url(&self, value: &str) -> Result<String, CliError> {
        if let Some(name) = &self.name {
            return Ok(name.clone());
        }
        let url = Url::parse(value).map_err(|_| {
            CliError::InvalidInput(
                "provide a target URL such as https://example.com/health, or an explicit --name"
                    .into(),
            )
        })?;
        let host = url.host_str().ok_or_else(|| {
            CliError::InvalidInput(
                "target URL needs a hostname to derive the name; supply --name".into(),
            )
        })?;
        Ok(format!("{host}{}", url.path().trim_end_matches('/')))
    }

    fn request(self, name: String, config: CreateConfig) -> CreateInput {
        CreateInput(InputSource::Generated(CreateRequest {
            name: Some(name),
            description: self.description,
            tags: Some(self.tags),
            config,
        }))
    }
}

impl HttpCreateArgs {
    fn into_input(self) -> Result<CreateInput, CliError> {
        if let Some(input) = self.common.file_input(MonitorKind::Http)? {
            return Ok(input);
        }
        let url = required(self.url, "URL")?;
        let name = self.common.name_for_url(&url)?;
        Ok(self.common.request(
            name,
            CreateConfig::Http {
                active: ActiveRequest {
                    url: Some(url),
                    method: self.method,
                    interval_secs: self.interval_secs,
                    ..ActiveRequest::default()
                },
                expected_status: None,
                response_time_max_ms: None,
            },
        ))
    }
}

impl FunctionCreateArgs {
    fn into_input(self) -> Result<CreateInput, CliError> {
        if let Some(input) = self.common.file_input(MonitorKind::Function)? {
            return Ok(input);
        }
        let url = required(self.url, "URL")?;
        let name = self.common.name_for_url(&url)?;
        let source = read_text_input(required(self.js_source_file.as_deref(), "--script")?)?;
        Ok(self.common.request(
            name,
            CreateConfig::Function {
                active: ActiveRequest {
                    url: Some(url),
                    interval_secs: self.interval_secs,
                    ..ActiveRequest::default()
                },
                js_source: Some(Secret::new(source)),
            },
        ))
    }
}

impl HeartbeatCreateArgs {
    fn into_input(self) -> Result<CreateInput, CliError> {
        if let Some(input) = self.common.file_input(MonitorKind::Heartbeat)? {
            return Ok(input);
        }
        let name = required(self.common.name.clone(), "--name")?;
        let schedule = match (self.period_secs, self.cron_expression) {
            (Some(period_secs), None) => Schedule::Interval {
                period_secs,
                grace_secs: self.grace_secs,
            },
            (None, Some(cron_expression)) => Schedule::Cron {
                cron_expression,
                grace_secs: self.grace_secs,
                timezone: None,
            },
            (Some(_), Some(_)) | (None, None) => {
                return Err(CliError::InvalidInput(
                    "supply --every 24h or --cron EXPRESSION when --file is absent".into(),
                ));
            }
        };
        let js_source = self
            .js_source_file
            .as_deref()
            .map(read_text_input)
            .transpose()?
            .map(Secret::new);
        Ok(self.common.request(
            name,
            CreateConfig::Heartbeat {
                schedule,
                js_source,
            },
        ))
    }
}
