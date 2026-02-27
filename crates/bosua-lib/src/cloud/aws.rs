//! AWS integration client.
//!
//! Provides EC2 instance management (start, stop, describe), security group
//! management, and region/zone operations using the `aws-sdk-ec2` crate.
//! Uses the configurable `AwsRegion` from `DynamicConfig` (default: ap-east-1).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::cloud::CloudClient;
use crate::config::dynamic::DynamicConfig;
use crate::errors::{BosuaError, Result};

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

/// Summary of an EC2 instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ec2Instance {
    pub instance_id: String,
    #[serde(default)]
    pub instance_type: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub public_ip: Option<String>,
    #[serde(default)]
    pub private_ip: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Summary of an EC2 security group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityGroup {
    pub group_id: String,
    pub group_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub vpc_id: Option<String>,
}

/// An AWS region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsRegionInfo {
    pub region_name: String,
    #[serde(default)]
    pub endpoint: String,
}

/// An AWS availability zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsZone {
    pub zone_name: String,
    pub region_name: String,
    #[serde(default)]
    pub state: String,
}

/// Result of an EC2 start/stop operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ec2StateChange {
    pub instance_id: String,
    pub previous_state: String,
    pub current_state: String,
}

/// An AMI (Amazon Machine Image) summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmiInfo {
    pub image_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub architecture: Option<String>,
}

// ---------------------------------------------------------------------------
// AwsClient
// ---------------------------------------------------------------------------

/// AWS client for EC2 instance management, security groups, and region/zone ops.
///
/// Uses `aws-sdk-ec2` under the hood. The region is derived from
/// `DynamicConfig.aws_region` (default: `ap-east-1`).
pub struct AwsClient {
    region: String,
    ec2_client: Option<aws_sdk_ec2::Client>,
}

impl AwsClient {
    /// Create a new `AwsClient` from `DynamicConfig`.
    pub fn new(config: &DynamicConfig) -> Self {
        Self {
            region: config.aws_region.clone(),
            ec2_client: None,
        }
    }

    /// Lazily initialise the EC2 SDK client.
    async fn ec2(&mut self) -> Result<&aws_sdk_ec2::Client> {
        if self.ec2_client.is_none() {
            let region = aws_sdk_ec2::config::Region::new(self.region.clone());
            let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(region)
                .load()
                .await;
            self.ec2_client = Some(aws_sdk_ec2::Client::new(&sdk_config));
        }
        Ok(self.ec2_client.as_ref().unwrap())
    }

    /// Update the region when `DynamicConfig` changes.
    pub fn update_from_config(&mut self, config: &DynamicConfig) {
        if self.region != config.aws_region {
            self.region = config.aws_region.clone();
            // Force re-creation of the SDK client on next call
            self.ec2_client = None;
        }
    }

    /// Get the currently configured region.
    pub fn region(&self) -> &str {
        &self.region
    }

    // -----------------------------------------------------------------------
    // EC2 instance management
    // -----------------------------------------------------------------------

    /// Describe (list) EC2 instances.
    pub async fn describe_instances(&mut self) -> Result<Vec<Ec2Instance>> {
        let client = self.ec2().await?;
        let resp = client
            .describe_instances()
            .send()
            .await
            .map_err(|e| BosuaError::Cloud {
                service: "aws".into(),
                message: format!("describe_instances failed: {e}"),
            })?;

        let mut instances = Vec::new();
        for reservation in resp.reservations() {
            for inst in reservation.instances() {
                let name = inst
                    .tags()
                    .iter()
                    .find(|t| t.key() == Some("Name"))
                    .and_then(|t| t.value().map(|v| v.to_string()));

                instances.push(Ec2Instance {
                    instance_id: inst.instance_id().unwrap_or_default().to_string(),
                    instance_type: inst
                        .instance_type()
                        .map(|t| t.as_str().to_string())
                        .unwrap_or_default(),
                    state: inst
                        .state()
                        .and_then(|s| s.name())
                        .map(|n| n.as_str().to_string())
                        .unwrap_or_default(),
                    public_ip: inst.public_ip_address().map(|s| s.to_string()),
                    private_ip: inst.private_ip_address().map(|s| s.to_string()),
                    name,
                });
            }
        }
        Ok(instances)
    }

    /// Start an EC2 instance.
    pub async fn start_instance(&mut self, instance_id: &str) -> Result<Ec2StateChange> {
        let client = self.ec2().await?;
        let resp = client
            .start_instances()
            .instance_ids(instance_id)
            .send()
            .await
            .map_err(|e| BosuaError::Cloud {
                service: "aws".into(),
                message: format!("start_instance failed: {e}"),
            })?;

        let change = resp
            .starting_instances()
            .first()
            .ok_or_else(|| BosuaError::Cloud {
                service: "aws".into(),
                message: "No state change returned".into(),
            })?;

        Ok(Ec2StateChange {
            instance_id: instance_id.to_string(),
            previous_state: change
                .previous_state()
                .and_then(|s| s.name())
                .map(|n| n.as_str().to_string())
                .unwrap_or_default(),
            current_state: change
                .current_state()
                .and_then(|s| s.name())
                .map(|n| n.as_str().to_string())
                .unwrap_or_default(),
        })
    }

    /// Stop an EC2 instance.
    pub async fn stop_instance(&mut self, instance_id: &str) -> Result<Ec2StateChange> {
        let client = self.ec2().await?;
        let resp = client
            .stop_instances()
            .instance_ids(instance_id)
            .send()
            .await
            .map_err(|e| BosuaError::Cloud {
                service: "aws".into(),
                message: format!("stop_instance failed: {e}"),
            })?;

        let change = resp
            .stopping_instances()
            .first()
            .ok_or_else(|| BosuaError::Cloud {
                service: "aws".into(),
                message: "No state change returned".into(),
            })?;

        Ok(Ec2StateChange {
            instance_id: instance_id.to_string(),
            previous_state: change
                .previous_state()
                .and_then(|s| s.name())
                .map(|n| n.as_str().to_string())
                .unwrap_or_default(),
            current_state: change
                .current_state()
                .and_then(|s| s.name())
                .map(|n| n.as_str().to_string())
                .unwrap_or_default(),
        })
    }

    // -----------------------------------------------------------------------
    // Security groups
    // -----------------------------------------------------------------------

    /// Describe (list) security groups.
    pub async fn describe_security_groups(&mut self) -> Result<Vec<SecurityGroup>> {
        let client = self.ec2().await?;
        let resp = client
            .describe_security_groups()
            .send()
            .await
            .map_err(|e| BosuaError::Cloud {
                service: "aws".into(),
                message: format!("describe_security_groups failed: {e}"),
            })?;

        let groups = resp
            .security_groups()
            .iter()
            .map(|sg| SecurityGroup {
                group_id: sg.group_id().unwrap_or_default().to_string(),
                group_name: sg.group_name().unwrap_or_default().to_string(),
                description: sg.description().unwrap_or_default().to_string(),
                vpc_id: sg.vpc_id().map(|s| s.to_string()),
            })
            .collect();

        Ok(groups)
    }

    // -----------------------------------------------------------------------
    // Regions and zones
    // -----------------------------------------------------------------------

    /// List available AWS regions.
    pub async fn describe_regions(&mut self) -> Result<Vec<AwsRegionInfo>> {
        let client = self.ec2().await?;
        let resp = client
            .describe_regions()
            .send()
            .await
            .map_err(|e| BosuaError::Cloud {
                service: "aws".into(),
                message: format!("describe_regions failed: {e}"),
            })?;

        let regions = resp
            .regions()
            .iter()
            .map(|r| AwsRegionInfo {
                region_name: r.region_name().unwrap_or_default().to_string(),
                endpoint: r.endpoint().unwrap_or_default().to_string(),
            })
            .collect();

        Ok(regions)
    }

    /// List availability zones in the current region.
    pub async fn describe_availability_zones(&mut self) -> Result<Vec<AwsZone>> {
        let client = self.ec2().await?;
        let resp = client
            .describe_availability_zones()
            .send()
            .await
            .map_err(|e| BosuaError::Cloud {
                service: "aws".into(),
                message: format!("describe_availability_zones failed: {e}"),
            })?;

        let zones = resp
            .availability_zones()
            .iter()
            .map(|z| AwsZone {
                zone_name: z.zone_name().unwrap_or_default().to_string(),
                region_name: z.region_name().unwrap_or_default().to_string(),
                state: z
                    .state()
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_default(),
            })
            .collect();

        Ok(zones)
    }

    /// Describe AMIs owned by the current account.
    pub async fn describe_images(&mut self) -> Result<Vec<AmiInfo>> {
        let client = self.ec2().await?;
        let resp = client
            .describe_images()
            .owners("self")
            .send()
            .await
            .map_err(|e| BosuaError::Cloud {
                service: "aws".into(),
                message: format!("describe_images failed: {e}"),
            })?;

        let images = resp
            .images()
            .iter()
            .map(|img| AmiInfo {
                image_id: img.image_id().unwrap_or_default().to_string(),
                name: img.name().map(|s| s.to_string()),
                state: img
                    .state()
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_default(),
                architecture: img.architecture().map(|a| a.as_str().to_string()),
            })
            .collect();

        Ok(images)
    }
}

// ---------------------------------------------------------------------------
// CloudClient trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl CloudClient for AwsClient {
    fn name(&self) -> &str {
        "AWS"
    }

    /// Authenticate by verifying that the SDK can reach the EC2 API.
    async fn authenticate(&self) -> Result<()> {
        // Authentication is handled by the AWS SDK credential chain
        // (env vars, config files, IAM roles). We verify connectivity
        // by attempting a lightweight API call.
        let region = aws_sdk_ec2::config::Region::new(self.region.clone());
        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region)
            .load()
            .await;
        let client = aws_sdk_ec2::Client::new(&sdk_config);

        client
            .describe_regions()
            .send()
            .await
            .map_err(|e| BosuaError::Auth(format!("AWS authentication check failed: {e}")))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_client_new() {
        let config = DynamicConfig::default();
        let client = AwsClient::new(&config);
        assert_eq!(client.region(), "ap-east-1");
    }

    #[test]
    fn test_aws_client_name() {
        let config = DynamicConfig::default();
        let client = AwsClient::new(&config);
        assert_eq!(client.name(), "AWS");
    }

    #[test]
    fn test_update_from_config() {
        let config = DynamicConfig::default();
        let mut client = AwsClient::new(&config);
        assert_eq!(client.region(), "ap-east-1");

        let mut updated = DynamicConfig::default();
        updated.aws_region = "us-west-2".into();
        client.update_from_config(&updated);
        assert_eq!(client.region(), "us-west-2");
    }

    #[test]
    fn test_ec2_instance_serialization() {
        let inst = Ec2Instance {
            instance_id: "i-1234567890abcdef0".into(),
            instance_type: "t2.micro".into(),
            state: "running".into(),
            public_ip: Some("54.1.2.3".into()),
            private_ip: Some("10.0.0.1".into()),
            name: Some("my-instance".into()),
        };
        let json = serde_json::to_string(&inst).unwrap();
        let deser: Ec2Instance = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.instance_id, "i-1234567890abcdef0");
        assert_eq!(deser.state, "running");
    }

    #[test]
    fn test_security_group_serialization() {
        let sg = SecurityGroup {
            group_id: "sg-12345".into(),
            group_name: "my-sg".into(),
            description: "My security group".into(),
            vpc_id: Some("vpc-abc".into()),
        };
        let json = serde_json::to_string(&sg).unwrap();
        let deser: SecurityGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.group_id, "sg-12345");
    }

    #[test]
    fn test_ec2_state_change_serialization() {
        let change = Ec2StateChange {
            instance_id: "i-123".into(),
            previous_state: "stopped".into(),
            current_state: "pending".into(),
        };
        let json = serde_json::to_string(&change).unwrap();
        let deser: Ec2StateChange = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.previous_state, "stopped");
        assert_eq!(deser.current_state, "pending");
    }

    #[test]
    fn test_ami_info_serialization() {
        let ami = AmiInfo {
            image_id: "ami-12345".into(),
            name: Some("my-ami".into()),
            state: "available".into(),
            architecture: Some("x86_64".into()),
        };
        let json = serde_json::to_string(&ami).unwrap();
        let deser: AmiInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.image_id, "ami-12345");
    }
}
