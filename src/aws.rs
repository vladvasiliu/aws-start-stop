use color_eyre::Result;

use aws_sdk_ec2::types::InstanceStateName;
use aws_sdk_ssm::types::ConnectionStatus;
use color_eyre::eyre::eyre;
use tokio::time::Duration;

pub struct Instance(aws_sdk_ec2::types::Instance);

impl Instance {
    pub fn state(&self) -> &InstanceStateName {
        self.0.state.as_ref().unwrap().name.as_ref().unwrap()
    }

    pub fn ipv6_address(&self) -> Option<&str> {
        self.0.ipv6_address()
    }

    pub fn ipv4_address_public(&self) -> Option<&str> {
        self.0.public_ip_address()
    }

    pub fn ipv4_address_private(&self) -> Option<&str> {
        self.0.private_ip_address()
    }
}

pub struct AwsEc2Client {
    client: aws_sdk_ec2::client::Client,
    instance_id: String,
    target_state: InstanceStateName,
    wait: Duration,
}

impl AwsEc2Client {
    pub fn new(
        client: aws_sdk_ec2::client::Client,
        instance_id: &str,
        target_state: InstanceStateName,
        wait: Duration,
    ) -> Self {
        Self {
            client,
            instance_id: instance_id.to_string(),
            target_state,
            wait,
        }
    }

    pub async fn get_instance(&self) -> Result<Instance> {
        let response = self
            .client
            .describe_instances()
            .instance_ids(&self.instance_id)
            .send()
            .await?;

        // Do a sanity check. There should be exactly one instance, no more, no less
        let mut reservations = response.reservations.unwrap_or_default();
        if reservations.is_empty() {
            return Err(eyre!("Instance not found"));
        } else if reservations.len() > 1 || response.next_token.is_some() {
            return Err(eyre!("Too many reservations returned"));
        }

        let reservation = reservations.pop().unwrap();

        let mut instance_vec = reservation.instances.unwrap_or_default();

        if instance_vec.is_empty() {
            return Err(eyre!("Instance not found"));
        } else if instance_vec.len() > 1 {
            return Err(eyre!("Too many instances returned"));
        }

        let instance = instance_vec.pop().unwrap();

        Ok(Instance(instance))
    }

    pub async fn start_instance(&self) -> Result<InstanceStateName> {
        let response = self
            .client
            .start_instances()
            .instance_ids(&self.instance_id)
            .send()
            .await?;

        // Sanity check
        let mut state_changes = response.starting_instances.unwrap_or_default();
        if state_changes.is_empty() {
            return Err(eyre!("Instance not found"));
        } else if state_changes.len() > 1 {
            return Err(eyre!("Too many instances started"));
        }

        let change = state_changes.pop().unwrap();
        if change.instance_id.unwrap() != self.instance_id {
            return Err(eyre!("Wrong instance started"));
        }

        let current_state = change.current_state.unwrap().name.unwrap();

        if current_state != InstanceStateName::Pending
            && current_state != InstanceStateName::Running
        {
            return Err(eyre!("Failed to start instance"));
        }

        Ok(current_state)
    }

    pub async fn stop_instance(&self) -> Result<InstanceStateName> {
        let response = self
            .client
            .stop_instances()
            .instance_ids(&self.instance_id)
            .send()
            .await?;

        // Sanity check
        let mut state_changes = response.stopping_instances.unwrap_or_default();
        if state_changes.is_empty() {
            return Err(eyre!("Instance not found"));
        } else if state_changes.len() > 1 {
            return Err(eyre!("Too many instances stopped"));
        }

        let change = state_changes.pop().unwrap();
        if change.instance_id.unwrap() != self.instance_id {
            return Err(eyre!("Wrong instance stopped"));
        }

        let current_state = change.current_state.unwrap().name.unwrap();

        if current_state != InstanceStateName::Stopping
            && current_state != InstanceStateName::Stopped
        {
            return Err(eyre!("Failed to stop instance"));
        }

        Ok(current_state)
    }

    pub async fn wait_for_state(&self) -> Result<Instance> {
        let mut wait_interval = tokio::time::interval(self.wait);
        loop {
            wait_interval.tick().await;
            let instance = self.get_instance().await?;
            if check_state(instance.state(), &self.target_state)? {
                return Ok(instance);
            }
        }
    }
}

/// Checks whether the current state is "before" or equal to the current state
///
/// If the current state is not before the desired state, return an error
/// If the state is before, but not equal to, the desired state, return `Ok(false)`
/// If the state is equal to the desired state, return `Ok(true)`
/// If the desired state is not `Running` or `Stopped`, return an error
/// Instance lifecycle docs:
/// https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-lifecycle.html
fn check_state(
    current_state: &InstanceStateName,
    desired_state: &InstanceStateName,
) -> Result<bool> {
    if *desired_state == InstanceStateName::Running {
        match current_state {
            InstanceStateName::Pending => Ok(false),
            InstanceStateName::Running => Ok(true),
            _ => Err(eyre!(
                "The instance is in an abnormal state. Current: {}, Desired: {}",
                current_state.as_str(),
                desired_state.as_str()
            )),
        }
    } else if *desired_state == InstanceStateName::Stopped {
        match current_state {
            InstanceStateName::Stopping => Ok(false),
            InstanceStateName::Stopped => Ok(true),
            _ => Err(eyre!(
                "The instance is in an abnormal state. Current: {}, Desired: {}",
                current_state.as_str(),
                desired_state.as_str()
            )),
        }
    } else {
        Err(eyre!(
            "The desired state ({}) is invalid",
            desired_state.as_str()
        ))
    }
}

pub struct AwsSsmClient {
    pub client: aws_sdk_ssm::client::Client,
    pub instance_id: String,
    pub wait: Duration,
}

impl AwsSsmClient {
    async fn get_connection_status(&self) -> Result<bool> {
        let res = self
            .client
            .get_connection_status()
            .target(&self.instance_id)
            .send()
            .await?;

        match res.status {
            None => Err(eyre!("SSM GetConnectionStatus returned nothing")),
            Some(status) => match status {
                ConnectionStatus::Connected => Ok(true),
                ConnectionStatus::NotConnected => Ok(false),
                _ => Err(eyre!(
                    "SSM GetConnectionStatus returned an unknown status: {}",
                    status.as_str()
                )),
            },
        }
    }

    pub async fn wait_for_connection(&self) -> Result<()> {
        let mut wait_interval = tokio::time::interval(self.wait);
        loop {
            wait_interval.tick().await;
            let connection_status = self.get_connection_status().await?;
            if connection_status {
                return Ok(());
            }
        }
    }
}
