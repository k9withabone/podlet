use std::{
    fmt::{self, Display, Formatter},
    net::{Ipv4Addr, Ipv6Addr},
    ops::Not,
    path::PathBuf,
};

use clap::{ArgAction, Args, Subcommand, ValueEnum};
use compose_spec::service::blkio_config::Weight;
use serde::{Serialize, Serializer};
use smart_default::SmartDefault;

use crate::{
    quadlet::{
        self,
        container::{Device, Dns, DnsEntry, Volume},
    },
    serde::skip_true,
};

use super::blkio_weight_parser;

/// [`Subcommand`]s for `podlet podman pod`.
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum Pod {
    /// Generate a Podman Quadlet `.pod` file.
    ///
    /// For details on options see:
    /// https://docs.podman.io/en/stable/markdown/podman-pod-create.1.html and
    /// https://docs.podman.io/en/stable/markdown/podman-systemd.unit.5.html#pod-units-pod
    #[allow(clippy::doc_markdown)]
    #[group(skip)]
    Create {
        #[command(flatten)]
        create: Create,
    },
}

impl Pod {
    /// The name (without extension) of the generated Quadlet file.
    pub fn name(&self) -> &str {
        let Self::Create { create } = self;
        &create.name
    }
}

impl From<Pod> for quadlet::Pod {
    fn from(Pod::Create { create }: Pod) -> Self {
        create.into()
    }
}

impl From<Pod> for quadlet::Resource {
    fn from(value: Pod) -> Self {
        quadlet::Pod::from(value).into()
    }
}

/// [`Args`] for `podman pod create`.
#[allow(clippy::doc_markdown)]
#[derive(Args, Debug, Clone, PartialEq)]
pub struct Create {
    /// Specify a custom network for the pod.
    ///
    /// Converts to "Network=MODE".
    ///
    /// Can be specified multiple times.
    #[arg(long, visible_alias = "net", value_name = "MODE")]
    network: Vec<String>,

    /// Publish a container's port, or a range of ports, within this pod to the host.
    ///
    /// **Note:** You must not publish ports of containers in the pod individually,
    /// but only by the pod itself.
    ///
    /// **Note:** This cannot be modified once the pod is created.
    ///
    /// Converts to "PublishPort=[[IP:][HOST_PORT]:]CONTAINER_PORT[/PROTOCOL]".
    ///
    /// Can be specified multiple times.
    #[arg(
        short,
        long,
        value_name = "[[IP:][HOST_PORT]:]CONTAINER_PORT[/PROTOCOL]"
    )]
    publish: Vec<String>,

    /// Mount a volume in the pod.
    ///
    /// Converts to "Volume=[[SOURCE-VOLUME|HOST-DIR:]CONTAINER-DIR[:OPTIONS]]".
    ///
    /// Can be specified multiple times.
    #[arg(
        short,
        long,
        value_name = "[[SOURCE-VOLUME|HOST-DIR:]CONTAINER-DIR[:OPTIONS]]"
    )]
    volume: Vec<Volume>,

    /// Converts to "PodmanArgs=ARGS".
    #[command(flatten)]
    podman_args: PodmanArgs,

    /// The name of the pod to create.
    ///
    /// This will be used as the name of the generated file when used with
    /// the --file option without a filename.
    #[arg(long, value_name = "NAME")]
    name: String,
}

impl From<Create> for quadlet::Pod {
    fn from(
        Create {
            network,
            publish: publish_port,
            volume,
            podman_args,
            // Name used when creating the `quadlet::File`.
            name: _,
        }: Create,
    ) -> Self {
        let podman_args = podman_args.to_string();

        Self {
            network,
            podman_args: (!podman_args.is_empty()).then_some(podman_args),
            pod_name: None,
            publish_port,
            volume,
        }
    }
}

/// [`Args`] for `podman pod create` (i.e. [`Create`]) that convert into `PodmanArgs=ARGS`.
#[derive(Args, Serialize, Debug, SmartDefault, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
struct PodmanArgs {
    /// Add a custom host-to-IP mapping.
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "HOST:IP")]
    add_host: Vec<String>,

    /// Block IO relative weight, between 10 and 1000.
    #[arg(long, value_name = "WEIGHT", value_parser = blkio_weight_parser())]
    blkio_weight: Option<Weight>,

    /// Block IO relative device weight.
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "DEVICE:WEIGHT")]
    blkio_weight_device: Vec<String>,

    /// Path to cgroups under which the cgroup for the pod will be created.
    #[arg(long, value_name = "PATH")]
    cgroup_parent: Option<PathBuf>,

    /// CPU shares (relative weight).
    #[arg(short, long, value_name = "SHARES")]
    cpu_shares: Option<u64>,

    /// Number of CPUs.
    #[arg(long, value_name = "NUMBER")]
    cpus: Option<f64>,

    /// CPUs in which to allow execution.
    #[arg(long, value_name = "NUMBER")]
    cpuset_cpus: Option<String>,

    /// Memory nodes (MEMs) in which to allow execution.
    #[arg(long, value_name = "NODES")]
    cpuset_mems: Option<String>,

    /// Add a device node from the host into the container.
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "HOST-DEVICE[:CONTAINER-DEVICE][:PERMISSIONS]")]
    device: Vec<Device>,

    /// Limit read rate (in bytes per second) from a device.
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "PATH:RATE")]
    device_read_bps: Vec<String>,

    /// Limit write rate (in bytes per second) to a device.
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "PATH:RATE")]
    device_write_bps: Vec<String>,

    /// Set custom DNS servers.
    ///
    /// Can be specified multiple times
    #[arg(long, value_name = "IP_ADDRESS")]
    #[serde(serialize_with = "serialize_dns")]
    // TODO: use `Dns` directly if clap ever supports custom collections (https://github.com/clap-rs/clap/issues/3114).
    dns: Vec<DnsEntry>,

    /// Set custom DNS options.
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "OPTION")]
    dns_option: Vec<String>,

    /// Set custom DNS search domains.
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "DOMAIN")]
    dns_search: Vec<String>,

    /// Set the exit policy of the pod when the last container exits.
    ///
    /// Only `stop` is supported as it is automatically set by Quadlet.
    #[arg(long, value_enum, default_value_t)]
    #[serde(skip)]
    exit_policy: ExitPolicy,

    /// GID map for the user namespace.
    ///
    /// Can be specified multiple times
    #[arg(
        long,
        value_name = "POD_GID:HOST_GID[:AMOUNT]",
        conflicts_with_all = ["userns", "subgidname"]
    )]
    gidmap: Vec<String>,

    /// GPU devices to add to the pod (`all` to pass all GPUs).
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "ENTRY")]
    gpus: Vec<String>,

    /// Set the hostname of the pod.
    #[arg(long, value_name = "NAME")]
    hostname: Option<String>,

    /// Create an infra container and associate it with the pod.
    ///
    /// Set by default and cannot be disabled as it is required by Quadlet.
    #[arg(long, action = ArgAction::SetTrue)]
    #[serde(skip)]
    infra: (),

    /// The command that is run to start the infra container.
    ///
    /// Default is "/pause".
    #[arg(long, value_name = "COMMAND")]
    infra_command: Option<String>,

    /// Custom image used for the infra container.
    ///
    /// By default, Podman builds a custom local image which does not require pulling down an image.
    #[arg(long, value_name = "IMAGE")]
    infra_image: Option<String>,

    /// Name used for the pod's infra container.
    #[arg(long, value_name = "NAME")]
    infra_name: Option<String>,

    /// Specify a static IPv4 address for the pod.
    #[arg(long, value_name = "IPV4")]
    ip: Option<Ipv4Addr>,

    /// Specify a static IPv6 address for the pod.
    #[arg(long, value_name = "IPV6")]
    ip6: Option<Ipv6Addr>,

    /// Add metadata to the pod.
    ///
    /// Can be specified multiple times
    #[arg(short, long, value_name = "KEY=VALUE")]
    label: Vec<String>,

    /// Read in a line-delimited file of labels
    #[arg(long, value_name = "FILE")]
    label_file: Option<PathBuf>,

    /// Pod network interface MAC address.
    #[arg(long, value_name = "ADDRESS")]
    mac_address: Option<String>,

    /// Memory limit.
    #[arg(short, long, value_name = "NUMBER[UNIT]")]
    memory: Option<String>,

    /// Limit value equal to memory plus swap.
    #[arg(long, value_name = "NUMBER[UNIT]")]
    memory_swap: Option<String>,

    /// Add a network-scoped alias for the pod.
    #[arg(long, value_name = "ALIAS")]
    network_alias: Option<String>,

    /// Do not create /etc/hosts for the pod.
    #[arg(long, conflicts_with = "add_host")]
    #[serde(skip_serializing_if = "Not::not")]
    no_hosts: bool,

    /// Set the PID namespace mode for the pod.
    #[arg(long)]
    pid: Option<String>,

    /// If another pod with the same name already exists, replace and remove it.
    ///
    /// Automatically set by Quadlet.
    #[arg(long)]
    #[serde(skip)]
    replace: bool,

    /// Restart policy to follow when containers exit.
    #[arg(long, value_name = "POLICY")]
    restart: Option<String>,

    /// Security options.
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "OPTION")]
    security_opt: Vec<String>,

    /// A comma-separated list of kernel namespaces to share.
    #[arg(long, value_name = "NAMESPACE")]
    share: Option<String>,

    /// Whether all containers entering the pod use the pod as their cgroup parent.
    #[arg(long, action = ArgAction::Set, default_value_t = true)]
    #[serde(skip_serializing_if = "skip_true")]
    #[default = true]
    share_parent: bool,

    /// Size of `/dev/shm`.
    #[arg(long, value_name = "NUMBER[UNIT]")]
    shm_size: Option<String>,

    /// Size of systemd-specific tmpfs mounts.
    #[arg(long, value_name = "NUMBER[UNIT]")]
    shm_size_systemd: Option<String>,

    /// Run the pod in a new user namespace using the map with `NAME` in the `/etc/subgid` file.
    #[arg(long, value_name = "NAME", conflicts_with_all = ["userns", "gidmap"])]
    subgidname: Option<String>,

    /// Run the pod in a new user namespace using the map with `NAME` in the `/etc/subuid` file.
    #[arg(long, value_name = "NAME", conflicts_with_all = ["userns", "uidmap"])]
    subuidname: Option<String>,

    /// Configure namespaced kernel parameters for all containers in the pod.
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "NAME=VALUE")]
    sysctl: Vec<String>,

    /// Run all containers in the pod in a new user namespace using the supplied mapping.
    ///
    /// Can be specified multiple times.
    #[arg(
        long,
        value_name = "CONTAINER_UID:FROM_UID[:AMOUNT]",
        conflicts_with_all = ["userns", "subuidname"]
    )]
    uidmap: Vec<String>,

    /// Set the user namespace mode for all the containers in the pod.
    #[arg(long, value_name = "MODE")]
    userns: Option<String>,

    /// Set the UTS namespace mode for the pod.
    #[arg(long, value_name = "MODE")]
    uts: Option<String>,

    /// Mount volumes from the specified container in the pod.
    ///
    /// Can be specified multiple times.
    #[arg(long, value_name = "CONTAINER[:OPTIONS]")]
    volumes_from: Vec<String>,
}

/// Serialize the `dns` field of [`PodmanArgs`] as [`Dns`].
fn serialize_dns<S: Serializer>(dns: &[DnsEntry], serializer: S) -> Result<S::Ok, S::Error> {
    dns.iter().copied().collect::<Dns>().serialize(serializer)
}

impl Display for PodmanArgs {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let args = crate::serde::args::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&args)
    }
}

/// Supported values of `podman pod create --exit-policy` for [`PodmanArgs`].
///
/// Only [`Stop`](Self::Stop) is supported because it automatically set by Quadlet for `.pod` files.
#[derive(ValueEnum, Debug, Default, Clone, Copy, PartialEq, Eq)]
enum ExitPolicy {
    /// The pod (including its infra container) is stopped when the last container exits.
    #[default]
    Stop,
}

impl Display for ExitPolicy {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let str = match self {
            Self::Stop => "stop",
        };
        f.write_str(str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn podman_args_default_display_empty() {
        let args = PodmanArgs::default();
        assert!(args.to_string().is_empty());
    }
}
