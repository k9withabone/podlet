use std::{borrow::Cow, fmt::Display, path::PathBuf};

use clap::Args;

#[derive(Args, Debug, Clone, PartialEq)]
pub struct QuadletOptions {
    /// Add Linux capabilities
    ///
    /// Converts to "AddCapability=CAPABILITY"
    ///
    /// Can be specified multiple times
    #[arg(long, value_name = "CAPABILITY")]
    cap_add: Vec<String>,

    /// Add a device node from the host into the container
    ///
    /// Converts to "AddDevice=DEVICE"
    ///
    /// Can be specified multiple times
    #[arg(long, value_name = "HOST-DEVICE[:CONTAINER-DEVICE][:PERMISSIONS]")]
    device: Vec<String>,

    /// Add an annotation to the container
    ///
    /// Converts to "Annotation=KEY=VALUE"
    ///
    /// Can be specified multiple times
    #[arg(long, value_name = "KEY=VALUE")]
    annotation: Vec<String>,

    /// The (optional) name of the container
    ///
    /// The default name is `systemd-%N`, where `%N` is the name of the service
    ///
    /// Converts to "ContainerName=NAME"
    #[arg(long)]
    name: Option<String>,

    /// Drop Linux capability from the default podman capability set
    ///
    /// If unspecified, the default is `all`
    ///
    /// Converts to "DropCapability=CAPABILITY"
    ///
    /// Can be specified multiple times
    #[arg(long, value_name = "CAPABILITY")]
    cap_drop: Vec<String>,

    /// Set environment variables in the container
    ///
    /// Converts to "Environment=ENV"
    ///
    /// Can be specified multiple times
    #[arg(short, long)]
    env: Vec<String>,

    /// Read in a line-delimited file of environment variables
    ///
    /// Converts to "EnvironmentFile=FILE"
    ///
    /// Can be specified multiple times
    #[arg(long, value_name = "FILE")]
    env_file: Vec<PathBuf>,

    /// Use the host environment in the container
    ///
    /// Converts to "EnvironmentHost=true"
    #[arg(long)]
    env_host: bool,

    /// Exposes a port, or a range of ports, from the host to the container
    ///
    /// Converts to "ExposeHostPort=PORT"
    ///
    /// Can be specified multiple times
    #[arg(long, value_name = "PORT")]
    expose: Vec<String>,

    /// Set one or more OCI labels on the container
    ///
    /// Converts to "Label=KEY=VALUE"
    ///
    /// Can be specified multiple times
    #[arg(short, long, value_name = "KEY=VALUE")]
    label: Vec<String>,

    /// Logging driver for the container
    ///
    /// The default is `passthrough`
    ///
    /// Converts to "LogDriver=DRIVER"
    #[arg(long, value_name = "DRIVER")]
    log_driver: Option<String>,

    /// Attach a filesystem mount to the container
    ///
    /// Converts to "Mount=MOUNT"
    ///
    /// Can be specified multiple times
    #[arg(long, value_name = "type=TYPE,TYPE-SPECIFIC-OPTION[,...]")]
    mount: Vec<String>,

    /// Specify a custom network for the container
    ///
    /// Converts to "Network=MODE"
    ///
    /// Can be specified multiple times
    #[arg(long, visible_alias = "net", value_name = "MODE")]
    network: Vec<String>,

    /// The rootfs to use for the container
    ///
    /// Converts to "Rootfs=PATH"
    #[arg(long, value_name = "PATH[:OPTIONS]")]
    rootfs: Option<String>,

    /// Publish a container's port, or a range of ports, to the host
    ///
    /// Converts to "PublishPort=PORT"
    ///
    /// Can be specified multiple times
    #[arg(
        short,
        long,
        value_name = "[[IP:][HOST_PORT]:]CONTAINER_PORT[/PROTOCOL]"
    )]
    publish: Vec<String>,

    /// Mount the container's root filesystem as read-only
    ///
    /// Converts to "ReadOnly=true"
    #[arg(long)]
    read_only: bool,

    /// Run the container in a new user namespace using the supplied UID mapping
    ///
    /// Converts to ""RemapUsers=manual" and "RemapUid=UID_MAP""
    #[arg(long, value_name = "CONTAINER_UID:FROM_UID:AMOUNT")]
    uidmap: Option<String>,

    /// Run the container in a new user namespace using the supplied GID mapping
    ///
    /// Converts to "RemapUsers=manual" and "RemapGid=GID_MAP"
    #[arg(long, value_name = "CONTAINER_GID:HOST_GID:AMOUNT")]
    gidmap: Option<String>,

    /// Run an init inside the container
    ///
    /// Converts to "RunInit=true"
    #[arg(long)]
    init: bool,

    /// Set the timezone in the container
    ///
    /// Converts to "Timezone=TIMEZONE"
    #[arg(long, value_name = "TIMEZONE")]
    tz: Option<String>,

    /// Set the UID and, optionally, the GID used in the container
    ///
    /// Converts to "User=UID" and "Group=GID"
    #[arg(short, long, value_name = "UID[:GID]")]
    user: Option<String>,

    /// Mount a volume in the container
    ///
    /// Converts to "Volume=VOLUME"
    ///
    /// Can be specified multiple times
    #[arg(
        short,
        long,
        value_name = "[[SOURCE-VOLUME|HOST-DIR:]CONTAINER-DIR[:OPTIONS]]"
    )]
    volume: Vec<String>,
}

impl Display for QuadletOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.cap_add.is_empty() {
            writeln!(f, "AddCapability={}", self.cap_add.join(" "))?;
        }

        for device in &self.device {
            writeln!(f, "AddDevice={device}")?;
        }

        if !self.annotation.is_empty() {
            writeln!(f, "Annotation={}", escape_spaces_join(&self.annotation))?;
        }

        if let Some(name) = &self.name {
            writeln!(f, "ContainerName={name}")?;
        }

        if !self.cap_drop.is_empty() {
            writeln!(f, "DropCapability={}", self.cap_drop.join(" "))?;
        }

        if !self.env.is_empty() {
            writeln!(f, "Environment={}", escape_spaces_join(&self.env))?;
        }

        for file in &self.env_file {
            writeln!(f, "EnvironmentFile={}", file.display())?;
        }

        if self.env_host {
            writeln!(f, "EnvironmentHost=true")?;
        }

        for port in &self.expose {
            writeln!(f, "ExposeHostPort={port}")?;
        }

        if !self.label.is_empty() {
            writeln!(f, "Label={}", escape_spaces_join(&self.label))?;
        }

        if let Some(log_driver) = &self.log_driver {
            writeln!(f, "LogDriver={log_driver}")?;
        }

        for mount in &self.mount {
            writeln!(f, "Mount={mount}")?;
        }

        for network in &self.network {
            writeln!(f, "Network={network}")?;
        }

        if let Some(rootfs) = &self.rootfs {
            writeln!(f, "Rootfs={rootfs}")?;
        }

        for port in &self.publish {
            writeln!(f, "PublishPort={port}")?;
        }

        if self.read_only {
            writeln!(f, "ReadOnly=true")?;
        }

        if self.uidmap.is_some() || self.gidmap.is_some() {
            writeln!(f, "RemapUsers=manual")?;
        }

        if let Some(uidmap) = &self.uidmap {
            writeln!(f, "RemapUid={uidmap}")?;
        }

        if let Some(gidmap) = &self.gidmap {
            writeln!(f, "RemapGid={gidmap}")?;
        }

        if self.init {
            writeln!(f, "RunInit=true")?;
        }

        if let Some(tz) = &self.tz {
            writeln!(f, "Timezone={tz}")?;
        }

        if let Some(user) = &self.user {
            if let Some((uid, gid)) = user.split_once(':') {
                writeln!(f, "User={uid}")?;
                writeln!(f, "Group={gid}")?;
            } else {
                writeln!(f, "User={user}")?;
            }
        }

        for volume in &self.volume {
            writeln!(f, "Volume={volume}")?;
        }

        Ok(())
    }
}

#[derive(Args, Debug, Clone, PartialEq)]
pub struct PodmanArgs {
    /// Add a custom host-to-IP mapping
    ///
    /// Can be specified multiple times
    #[arg(long, value_name = "HOST:IP")]
    add_host: Vec<String>,

    /// Override the architecture of the image to be pulled
    ///
    /// Defaults to hosts architecture
    #[arg(long)]
    arch: Option<String>,

    /// Attach to STDIN, STDOUT, or STDERR
    #[arg(short, long, value_name = "STDIN | STDOUT | STDERR")]
    attach: Vec<String>,

    /// Path of the authentication file
    ///
    /// Default is `${XDG_RUNTIME_DIR}/containers/auth.json`
    #[arg(long, value_name = "PATH")]
    authfile: Option<PathBuf>,

    /// Block IO relative weight, between 10 and 1000
    #[arg(long, value_name = "WEIGHT")]
    blkio_weight: Option<u16>,
}

impl PodmanArgs {
    /// Whether all fields are empty
    fn is_empty(&self) -> bool {
        self.add_host.is_empty()
            && self.arch.is_none()
            && self.attach.is_empty()
            && self.authfile.is_none()
            && self.blkio_weight.is_none()
    }
}

impl Display for PodmanArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            Ok(())
        } else {
            let add_host = self.add_host.iter().map(|host| ["--add-host", host]);

            let arch = self.arch.as_ref().map(|arch| ["--arch", arch]);

            let attach = self.attach.iter().map(|attach| ["--attach", attach]);

            let authfile = self.authfile.as_ref().map(|file| file.to_string_lossy());
            let authfile = authfile.as_ref().map(|file| ["--authfile", file]);

            let blkio_weight = self.blkio_weight.map(|weight| weight.to_string());
            let blkio_weight = blkio_weight
                .as_ref()
                .map(|weight| ["--blkio-weight", weight]);

            let args_iter = add_host
                .chain(arch)
                .chain(attach)
                .chain(authfile)
                .chain(blkio_weight)
                .flatten();

            writeln!(f, "PodmanArgs={}", shlex::join(args_iter))
        }
    }
}

fn escape_spaces_join<'a>(words: impl IntoIterator<Item = &'a String>) -> String {
    words
        .into_iter()
        .map(|word| {
            if word.contains(' ') {
                format!("\"{word}\"").into()
            } else {
                word.into()
            }
        })
        .collect::<Vec<Cow<_>>>()
        .join(" ")
}
