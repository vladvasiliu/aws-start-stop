use clap::{command, AppSettings, Arg, PossibleValue};
use color_eyre::{eyre::eyre, Result};

#[derive(Debug, PartialEq, Clone)]
pub enum Action {
    Start,
    Stop,
}

impl clap::ValueEnum for Action {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Start, Self::Stop]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue<'a>> {
        match self {
            Self::Start => Some(PossibleValue::new("start")),
            Self::Stop => Some(PossibleValue::new("stop")),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub action: Action,
    pub instance_id: String,
    pub timeout: u64,
    pub wait_for_ssm: bool,
}

impl Config {
    pub fn from_args() -> Result<Self> {
        let matches = command!()
            .setting(AppSettings::DeriveDisplayOrder)
            .term_width(120)
            .args(&[
                Arg::new("action")
                    .takes_value(true)
                    .ignore_case(true)
                    .value_name("ACTION")
                    .required(true)
                    .value_parser(clap::builder::EnumValueParser::<Action>::new())
                    .help("Action"),
                Arg::new("instance")
                    .takes_value(true)
                    .value_name("INSTANCE_ID")
                    .required(true)
                    .value_parser(clap::builder::NonEmptyStringValueParser::new())
                    .help("Instance ID"),
                Arg::new("timeout")
                    .short('t')
                    .long("timeout")
                    .takes_value(true)
                    .value_name("TIMEOUT")
                    .required(false)
                    .multiple_values(false)
                    .value_parser(clap::builder::RangedU64ValueParser::<u64>::new())
                    .default_value("120")
                    .help("How long to wait for the action to complete"),
                Arg::new("wait-for-ssm")
                    .short('s')
                    .long("wait-for-ssm")
                    .takes_value(false)
                    .required(false)
                    .help("Wait for the instance to connect to SSM"),
            ])
            .get_matches();

        let action = matches
            .get_one::<Action>("action")
            .ok_or_else(|| eyre!("Missing action"))?
            .clone();
        let instance_id = matches
            .get_one::<String>("instance")
            .ok_or_else(|| eyre!("Missing instance id"))?
            .clone();
        let timeout = *matches
            .get_one::<u64>("timeout")
            .ok_or_else(|| eyre!("Missing timeout"))?;
        let wait_for_ssm = matches.contains_id("wait-for-ssm");

        Ok(Self {
            action,
            instance_id,
            timeout,
            wait_for_ssm,
        })
    }
}
