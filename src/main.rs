mod aws;
mod config;

use crate::aws::AwsClient;
use crate::config::{Action, Config};
use aws_sdk_ec2::model::InstanceStateName;
use color_eyre::Result;
use std::process::exit;
use tokio::time::{timeout, Duration};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let config = Config::from_args();

    let aws_config = aws_config::load_from_env().await;
    let aws_ec2_client = aws_sdk_ec2::client::Client::new(&aws_config);

    let desired_state = match config.action {
        Action::Stop => InstanceStateName::Stopped,
        Action::Start => InstanceStateName::Running,
    };

    let aws_client = AwsClient::new(
        aws_ec2_client,
        &config.instance_id,
        desired_state,
        Duration::from_secs(10),
    );

    let res = timeout(
        Duration::from_secs(config.timeout),
        work(aws_client, config.action),
    )
    .await;

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

async fn work(aws_client: AwsClient, action: Action) -> Result<()> {
    match action {
        Action::Start => {
            println!("Starting instance...");
            aws_client.start_instance().await?
        }
        Action::Stop => {
            println!("Stopping instance...");
            aws_client.stop_instance().await?
        }
    };

    let instance = aws_client.wait_for_state().await?;

    if action == Action::Start {
        println!("started instance");
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
