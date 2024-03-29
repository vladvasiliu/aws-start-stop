mod aws;
mod config;

use crate::aws::{AwsEc2Client, AwsSsmClient};
use crate::config::{Action, Config};
use aws_config::BehaviorVersion;
use aws_sdk_ec2::types::InstanceStateName;
use color_eyre::Result;
use std::process::exit;
use tokio::time::{timeout, Duration};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let config = Config::from_args()?;

    let res = timeout(Duration::from_secs(config.timeout), work(config)).await;

    match res {
        Err(_) => {
            println!("Failed to start instance: timeout");
            exit(1)
        }
        Ok(result) => match result {
            Ok(()) => {}
            Err(err) => {
                println!("Failed to start instance: {}", err);
                exit(2)
            }
        },
    }

    Ok(())
}

async fn work(config: Config) -> Result<()> {
    let desired_state = match config.action {
        Action::Stop => InstanceStateName::Stopped,
        Action::Start => InstanceStateName::Running,
    };
    let aws_config = aws_config::load_defaults(BehaviorVersion::latest()).await;

    let aws_ec2_client = AwsEc2Client::new(
        aws_sdk_ec2::client::Client::new(&aws_config),
        &config.instance_id,
        desired_state,
        Duration::from_secs(10),
    );

    match config.action {
        Action::Start => {
            println!("Starting instance...");
            aws_ec2_client.start_instance().await?
        }
        Action::Stop => {
            println!("Stopping instance...");
            aws_ec2_client.stop_instance().await?
        }
    };

    let instance = aws_ec2_client.wait_for_state().await?;

    if config.action == Action::Start {
        if config.wait_for_ssm {
            println!("Waiting for connection to SSM...");
            let aws_ssm_client = AwsSsmClient {
                client: aws_sdk_ssm::client::Client::new(&aws_config),
                instance_id: config.instance_id,
                wait: Duration::from_secs(10),
            };
            if let Err(e) = aws_ssm_client.wait_for_connection().await {
                println!("Failed to retrieve SSM connection status: {}", e);
            }
        }

        println!("Started instance:");
        println!(
            "\t public IPv4: {}",
            instance.ipv4_address_public().unwrap_or("None")
        );
        println!(
            "\tprivate IPv4: {}",
            instance.ipv4_address_private().unwrap_or("None")
        );
        println!(
            "\t        IPv6: {}",
            instance.ipv6_address().unwrap_or("None")
        );
    } else {
        println!("stopped instance");
    }

    Ok(())
}
