use clap::{command, AppSettings, Arg};
use color_eyre::eyre::eyre;
use color_eyre::Report;
use std::str::FromStr;

#[derive(Debug, std::cmp::PartialEq)]
pub enum Action {
    Start,
    Stop,
}

impl Action {
    pub fn arg_values() -> [&'static str; 2] {
        ["start", "stop"]
    }
}

impl FromStr for Action {
    type Err = Report;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "start" => Ok(Self::Start),
            "stop" => Ok(Self::Stop),
            _ => Err(eyre!("Incorrect action")),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub action: Action,
    pub instance_id: String,
    pub timeout: u64,
}

impl Config {
    pub fn from_args() -> Self {
        let matches = command!()
            .setting(AppSettings::DeriveDisplayOrder)
            .term_width(120)
            .args(&[
                Arg::new("action")
                    .takes_value(true)
                    .ignore_case(true)
                    .value_name("ACTION")
                    .possible_values(Action::arg_values())
                    .required(true)
                    .forbid_empty_values(true)
                    .help("Action"),
                Arg::new("instance")
                    .takes_value(true)
                    .value_name("INSTANCE_ID")
                    .required(true)
                    .forbid_empty_values(true)
                    .help("Instance ID"),
                Arg::new("timeout")
                    .short('t')
                    .long("timeout")
                    .takes_value(true)
                    .value_name("TIMEOUT")
                    .required(false)
                    .multiple_occurrences(false)
                    .multiple_values(false)
                    .forbid_empty_values(true)
                    .default_value("120")
                    .help("How long to wait for the action to complete"),
            ])
            .get_matches();

        let action = matches.value_of_t_or_exit("action");
        let instance_id = matches.value_of_t_or_exit("instance");
        let timeout = matches.value_of_t_or_exit("timeout");

        Self {
            action,
            instance_id,
            timeout,
        }
    }
}
